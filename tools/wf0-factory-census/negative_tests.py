#!/usr/bin/env python3
"""Deterministic DX0 identity, admission, invalidation, and recovery proofs."""

from __future__ import annotations

import ast
import copy
import json
import pathlib
import re
import tempfile
from typing import Any

from artifacts import (
    accepted_fixture_identity, accepted_fixture_identity_sha256,
    create_source_handoff, verify_fixture_store, verify_source_handoff,
)
from common import (
    DX0_AGAIN_BUNDLE_MANIFEST_SHA256, DX0_AGAIN_MODULE_SHA256,
    DX0_COMPLETE_SOURCE_SCHEMA, DX0_DECK_EXECUTION_PATHS,
    DX0_EVIDENCE_RENDERER_SCHEMA, DX0_PLAN_ID, DX0_REF, DX0_RENDERER_PATHS,
    DX0_RESULT_SCHEMA, DX0_SOURCE_PATHS, DX0_TRANSACTION_SCHEMA,
    DX0_TRANSACTION_STATE_SCHEMA, DX0_WINDOWS_BUILD_PATHS,
    RUNNER_DIGEST, canonical_json, dx0_closed_plan, dx0_complete_source,
    dx0_deck_execution_input, dx0_evidence_renderer, dx0_identity_sha256,
    dx0_source_role, dx0_validate_plan, dx0_windows_build_input, fail,
    sha256_bytes,
)
from evidence import (
    publish_packet, render_packet, result_admission_receipt, validate_packet,
    validate_pc0_packet_value, validate_result,
)
from run import validate_retained_result, validate_retained_result_file
from verify import audio_method_verifier_regression, scanner_component_call_surface


def _replace_blob(value: dict[str, Any], path: str, blob: str = "f" * 40) -> dict[str, Any]:
    changed = copy.deepcopy(value)
    records = changed.get("records", [])
    found = 0
    for record in records:
        if record.get("path") == path:
            record["git_blob"] = blob
            found += 1
    if found != 1:
        fail(f"DX0 deterministic mutation path is not in identity: {path}")
    return changed


def _role(seed: str) -> dict[str, Any]:
    return {
        "identity_sha256": seed * 64, "commit": seed * 40, "tree": seed * 40,
        "parent": "0" * 40, "ref": DX0_REF,
        "manifest_sha256": seed * 64,
    }


def synthetic_valid_result(plan_sha: str, *,
                           producer_role: dict[str, Any] | None = None,
                           execution_role: dict[str, Any] | None = None) -> dict[str, Any]:
    producer_source = producer_role or _role("1")
    execution_source = execution_role or _role("2")
    positive = {
        "query_result_u32_hex": "00000000", "query_output_nonnull": True,
        "query_tuple_consistent": True, "interface_release_reference_count": 1,
        "audio_processor_method_called": False, "fixture": "AGain VST3",
        "interface_logical_iid": "42043F99B7DA453CA569E79D9AAEC33D",
        "interface_raw_windows_tuid": "993F0442DAB73C45A569E79D9AAEC33D",
    }
    calls = {"paired_call_ledger": True, "started_count": 22,
             "completed_count": 22, "last_in_flight_operation": None,
             "query_audio_processor_count": 1, "release_audio_processor_count": 1,
             "ledger_overflowed": False}
    quiescence = {"interface_quiescence": True, "object_quiescence": True}
    shutdown = {"component_release_reference_count": 0,
                "reverse_factory_release": True, "exit_dll": True,
                "free_library": True, "scanner_completed": True,
                "clean_in_process_shutdown": True}
    cleanup = {"owned_descendant_count": 0, "process_group_empty": True,
               "environment_retired": True, "stage_absent": True}
    protected = {"pre_sha256": "a" * 64, "post_sha256": "a" * 64,
                 "equal": True, "comparison_completed": True}
    execution_input_sha = "b" * 64
    return {
        "schema": DX0_RESULT_SCHEMA, "operation_nonce": "c" * 32,
        "artifact_producer_source": producer_source,
        "deck_execution_source": execution_source,
        "execution_input": {
            "identity_sha256": execution_input_sha,
            "schema": "linux-vst-bridge-dx0-deck-execution-input/v1",
            "proof_plan_sha256": plan_sha, "runtime_proton_sha256": RUNNER_DIGEST,
            "source_handoff_ref":
                f"refs/handoff/dx0-source/{execution_source['commit']}",
            "detached_worktree_commit": execution_source["commit"],
            "host_artifact_manifest_sha256": "d" * 64,
            "accepted_fixture_identity_sha256": "e" * 64,
        },
        "host_artifact": {
            "windows_build_input_sha256": "f" * 64, "workflow_run_id": 1,
            "run_attempt": 1, "artifact_id": 1, "manifest_sha256": "d" * 64,
            "build_receipt_sha256": "1" * 64, "mac_custody_receipt_sha256": "2" * 64,
        },
        "accepted_fixture": {
            "identity_sha256": "e" * 64,
            "bundle_manifest_sha256": DX0_AGAIN_BUNDLE_MANIFEST_SHA256,
            "module_sha256": DX0_AGAIN_MODULE_SHA256,
            "mac_store_receipt_sha256": "3" * 64,
            "deck_store_receipt_sha256": "3" * 64,
        },
        "source_handoff": {
            "bundle_sha256": "4" * 64, "receipt_sha256": "5" * 64,
            "advertised_ref":
                f"refs/handoff/dx0-source/{execution_source['commit']}",
            "worktree_commit": execution_source["commit"], "worktree_clean": True,
        },
        "closed_plan": {
            "plan_id": DX0_PLAN_ID, "sha256": plan_sha,
            "expected_result": "wa0-positive-interface-lease-complete-v1",
            "live_exercise_ceiling": 1,
        },
        "original_observation": {
            "run_id": "6" * 32, "phase_nonce": "7" * 32,
            "event_stream_sha256": "8" * 64,
            "completion_disposition": "completed",
            "started_utc": "2026-09-02T00:00:00Z",
            "completed_utc": "2026-09-02T00:01:00Z",
        },
        "positive_result": positive, "call_facts": calls,
        "quiescence": quiescence, "shutdown": shutdown,
        "cleanup": cleanup, "protected_state": protected,
        "integrity": {
            "positive_result_sha256": sha256_bytes(canonical_json(positive)),
            "call_facts_sha256": sha256_bytes(canonical_json(calls)),
            "quiescence_sha256": sha256_bytes(canonical_json(quiescence)),
            "shutdown_sha256": sha256_bytes(canonical_json(shutdown)),
            "cleanup_sha256": sha256_bytes(canonical_json(cleanup)),
            "protected_state_sha256": sha256_bytes(canonical_json(protected)),
        },
    }


DX0_TEST_OPERATIONS = {
    "derive_identities", "plan_external_work", "freeze_source", "verify_fixture",
    "reuse_or_produce_host", "custody_host_artifact", "create_source_handoff",
    "transfer_and_admit_deck_inputs", "execute_deck_batch",
    "retrieve_and_retain_result", "render_and_validate_evidence",
    "close_transaction",
}


def _phase_receipt(name: str, inputs: dict[str, Any], outputs: dict[str, Any],
                   *, nonce: str = "9" * 32,
                   disposition: str = "completed") -> dict[str, Any]:
    return {
        "phase": name, "phase_nonce": nonce, "disposition": disposition,
        "input_sha256": sha256_bytes(canonical_json(inputs)),
        "inputs": inputs, "outputs": outputs,
    }


def synthetic_original_state(result: dict[str, Any]) -> dict[str, Any]:
    phases = {
        name: _phase_receipt(name, {}, {}) for name in DX0_TEST_OPERATIONS
    }
    execute_inputs = {
        "deck_execution_input_sha256": result["execution_input"]["identity_sha256"],
        "proof_plan_sha256": result["closed_plan"]["sha256"],
        "phase_nonce": result["original_observation"]["phase_nonce"],
    }
    phases["execute_deck_batch"] = _phase_receipt(
        "execute_deck_batch", execute_inputs,
        {"retained_result_sha256": sha256_bytes(canonical_json(result))},
        nonce=result["original_observation"]["phase_nonce"],
    )
    phases["retrieve_and_retain_result"] = _phase_receipt(
        "retrieve_and_retain_result", {}, {
            "retained_result_sha256": sha256_bytes(canonical_json(result)),
        },
    )
    phases["transfer_and_admit_deck_inputs"] = _phase_receipt(
        "transfer_and_admit_deck_inputs", {}, {
            "source_ref": result["source_handoff"]["advertised_ref"],
            "detached_worktree_commit": result["deck_execution_source"]["commit"],
            "deck_github_operations": 0,
        }, disposition="reused",
    )
    return {
        "schema": DX0_TRANSACTION_STATE_SCHEMA,
        "operation_nonce": result["operation_nonce"],
        "transaction_key": sha256_bytes(canonical_json({
            "source": result["deck_execution_source"],
            "plan_sha256": result["closed_plan"]["sha256"],
        })),
        "state": "transaction_complete",
        "source": result["deck_execution_source"],
        "plan_sha256": result["closed_plan"]["sha256"],
        "created_utc": "2026-09-02T00:00:00Z",
        "run_invocation_count": 1,
        "phases": phases,
        "effect_counts": {
            "windows_builds": 0, "artifact_downloads": 0,
            "custody_operations": 0, "artifact_transfers": 0,
            "source_transfers": 1, "deck_executions": 1,
            "evidence_renders": 1,
        },
    }


def _expect_rejected(value: dict[str, Any]) -> bool:
    try:
        validate_result(value)
    except RuntimeError:
        return True
    return False


def _malformed_result_mutations() -> dict[str, Any]:
    return {
        "missing_cleanup": lambda item: item.pop("cleanup"),
        "failed_query": lambda item: item["positive_result"].update(
            query_result_u32_hex="80004002"
        ),
        "nonzero_descendants": lambda item: item["cleanup"].update(
            owned_descendant_count=1
        ),
        "environment_not_retired": lambda item: item["cleanup"].update(
            environment_retired=False
        ),
        "protected_mismatch": lambda item: item["protected_state"].update(
            equal=False
        ),
        "wrong_type": lambda item: item["host_artifact"].update(artifact_id="1"),
        "boolean_integer": lambda item: item["shutdown"].update(
            component_release_reference_count=False
        ),
        "invalid_timestamp": lambda item: item["original_observation"].update(
            completed_utc="not-a-time"
        ),
        "wrong_handoff_ref": lambda item: item["source_handoff"].update(
            advertised_ref="refs/handoff/dx0-source/" + "0" * 40
        ),
        "wrong_detached_worktree": lambda item: item["execution_input"].update(
            detached_worktree_commit="0" * 40
        ),
    }


def _validator_rejects(validator: Any, value: dict[str, Any]) -> bool:
    try:
        validator(value)
    except RuntimeError:
        return True
    return False


def _repo_python_path(node: ast.AST) -> pathlib.PurePosixPath | None:
    if (isinstance(node, ast.Call) and isinstance(node.func, ast.Name)
            and node.func.id == "repo_root" and not node.args
            and not node.keywords):
        return pathlib.PurePosixPath(".")
    if isinstance(node, ast.Constant) and isinstance(node.value, str):
        return pathlib.PurePosixPath(node.value)
    if isinstance(node, ast.BinOp) and isinstance(node.op, ast.Div):
        left = _repo_python_path(node.left)
        right = _repo_python_path(node.right)
        if left is not None and right is not None:
            return left / right
    return None


def deck_execution_import_closure(source_root: pathlib.Path) -> tuple[str, ...]:
    """Resolve the actual local imports and repository Python executions."""
    module_root = source_root / "tools/wf0-factory-census"
    local_modules = {
        path.stem: path.relative_to(source_root)
        for path in module_root.glob("*.py")
        if path.is_file() and not path.is_symlink()
    }
    entry = pathlib.PurePosixPath("tools/wf0-factory-census/run.py")
    queue = [entry]
    closure: set[pathlib.PurePosixPath] = set()
    while queue:
        relative = queue.pop(0)
        if relative in closure:
            continue
        path = source_root / relative
        if (not path.is_file() or path.is_symlink()
                or path.suffix != ".py"):
            fail(f"DX0 Deck import-closure path is unsafe: {relative}")
        closure.add(relative)
        tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(relative))
        imported_names: set[str] = set()
        assignments: dict[str, pathlib.PurePosixPath] = {}
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                imported_names.update(alias.name.split(".", 1)[0] for alias in node.names)
            elif isinstance(node, ast.ImportFrom) and node.module:
                imported_names.add(node.module.split(".", 1)[0])
            elif (isinstance(node, ast.Assign) and len(node.targets) == 1
                  and isinstance(node.targets[0], ast.Name)):
                selected = _repo_python_path(node.value)
                if (selected is not None and selected.suffix == ".py"
                        and not selected.is_absolute() and ".." not in selected.parts):
                    assignments[node.targets[0].id] = selected
        for name in sorted(imported_names):
            imported = local_modules.get(name)
            if imported is not None and imported not in closure:
                queue.append(imported)
        for node in ast.walk(tree):
            if not isinstance(node, ast.Call) or not node.args:
                continue
            called = (
                node.func.id if isinstance(node.func, ast.Name)
                else node.func.attr if isinstance(node.func, ast.Attribute)
                else ""
            )
            if called not in {"command", "Popen", "run"}:
                continue
            for nested in ast.walk(node.args[0]):
                if isinstance(nested, ast.Name) and nested.id in assignments:
                    executed = assignments[nested.id]
                    if executed not in closure:
                        queue.append(executed)
    return tuple(sorted(str(path) for path in closure))


class FakeGitHub:
    def __init__(self, source: str, nonce: str) -> None:
        self.source = source
        self.nonce = nonce

    def json(self, route: str) -> dict[str, Any]:
        return {"workflow_runs": [{"id": 73, "head_sha": self.source,
                                   "event": "workflow_dispatch",
                                   "display_title": f"DX0 host fixture nonce {self.nonce}"}]}


class SettlingRunGitHub:
    def __init__(self, source: str, nonce: str, build_input_sha256: str) -> None:
        self.source = source
        self.nonce = nonce
        self.build_input_sha256 = build_input_sha256
        self.calls = 0

    def json(self, route: str) -> dict[str, Any]:
        self.calls += 1
        settled = self.calls > 1
        return {
            "id": 73, "event": "workflow_dispatch",
            "head_branch": "codex/dx0-split-build-identity-proof-transaction",
            "head_sha": self.source,
            "path": ".github/workflows/wf0-windows-msvc-build.yml",
            "display_title": (
                f"DX0 host {self.build_input_sha256} nonce {self.nonce}"
                if settled else "temporarily unsettled"
            ),
            "repository": {"full_name": "kasselvania/Linux-VST-bridge"},
            "run_attempt": 1, "status": "queued", "conclusion": None,
        }


class NeverCompletesGitHub:
    def __init__(self, source: str, nonce: str, build_input_sha256: str) -> None:
        self.source = source
        self.nonce = nonce
        self.build_input_sha256 = build_input_sha256
        self.calls = 0

    def json(self, route: str) -> dict[str, Any]:
        self.calls += 1
        return {
            "id": 74, "event": "workflow_dispatch",
            "head_branch": "codex/dx0-split-build-identity-proof-transaction",
            "head_sha": self.source,
            "path": ".github/workflows/wf0-windows-msvc-build.yml",
            "display_title": (
                f"DX0 host {self.build_input_sha256} nonce {self.nonce}"
            ),
            "repository": {"full_name": "kasselvania/Linux-VST-bridge"},
            "run_attempt": 1, "status": "queued", "conclusion": None,
        }


def deterministic_tests(source_root: pathlib.Path, *, source_commit: str,
                        driver_class: type[Any], result_recovery: Any,
                        store_join_validator: Any,
                        observation_state_validator: Any,
                        execution_writer_recovery: Any,
                        deck_result_recorder: Any,
                        transaction_keys: set[str],
                        seed_fixture_acquirer: Any) -> dict[str, Any]:
    current = dx0_complete_source(source_commit)
    if current["record_count"] != 10 or tuple(record["path"] for record in current["records"]) != DX0_SOURCE_PATHS:
        fail("DX0 deterministic source-roster proof failed")
    build = dx0_windows_build_input(source_commit)
    plan = dx0_closed_plan(DX0_PLAN_ID)
    plan_sha = dx0_identity_sha256(plan)
    fixture_sha = accepted_fixture_identity_sha256()
    deck = dx0_deck_execution_input(source_commit, "9" * 64, fixture_sha, plan_sha)
    renderer = dx0_evidence_renderer(source_commit)
    import_closure = deck_execution_import_closure(source_root)
    if (import_closure != DX0_DECK_EXECUTION_PATHS
            or "tools/wf0-factory-census/evidence.py" in import_closure):
        fail("DX0 transitive Deck import closure differs")

    historical_a = dx0_windows_build_input("4a5ff302acc4927142003e29bd401368920b275b")
    historical_b = dx0_windows_build_input("24b7e6da7e29a5bd358097a6b89c5c59b747c413")
    if historical_a != historical_b:
        fail("DX0 historical evidence-only sources do not reproduce one build identity")

    complete_renderer = _replace_blob(current, "tools/wf0-factory-census/evidence.py")
    renderer_changed = _replace_blob(renderer, "tools/wf0-factory-census/evidence.py")
    if (dx0_identity_sha256(complete_renderer) == dx0_identity_sha256(current)
            or dx0_identity_sha256(renderer_changed) == dx0_identity_sha256(renderer)
            or "tools/wf0-factory-census/evidence.py" in DX0_WINDOWS_BUILD_PATHS
            or "tools/wf0-factory-census/evidence.py" in DX0_DECK_EXECUTION_PATHS):
        fail("DX0 renderer-only invalidation classification failed")

    complete_driver = _replace_blob(current, "tools/host-proof.py")
    if (dx0_identity_sha256(complete_driver) == dx0_identity_sha256(current)
            or "tools/host-proof.py" in DX0_WINDOWS_BUILD_PATHS
            or "tools/host-proof.py" in DX0_DECK_EXECUTION_PATHS
            or "tools/host-proof.py" in DX0_RENDERER_PATHS):
        fail("DX0 Mac-driver-only invalidation classification failed")

    deck_changed = _replace_blob(deck, "tools/wf0-factory-census/run.py")
    if (dx0_identity_sha256(deck_changed) == dx0_identity_sha256(deck)
            or "tools/wf0-factory-census/run.py" in DX0_WINDOWS_BUILD_PATHS):
        fail("DX0 Deck-only invalidation classification failed")
    build_changed = _replace_blob(build, ".github/workflows/wf0-windows-msvc-build.yml")
    if dx0_identity_sha256(build_changed) == dx0_identity_sha256(build):
        fail("DX0 Windows-build invalidation classification failed")

    fixture = accepted_fixture_identity()
    call_surface = scanner_component_call_surface(source_root)
    audio_method_regression = audio_method_verifier_regression()
    workflow_text = (source_root / ".github/workflows/wf0-windows-msvc-build.yml").read_text(
        encoding="utf-8"
    )
    build_text = (source_root / "tools/wf0-factory-census/build.py").read_text(
        encoding="utf-8"
    )
    dx0_build_body = build_text.split("def build_dx0_workflow", 1)[1].split(
        "\ndef build_workflow", 1
    )[0]
    if (fixture["bundle_manifest"]["sha256"] != DX0_AGAIN_BUNDLE_MANIFEST_SHA256
            or build["build_contract"]["targets"] != ["wf0-factory-probe"]
            or build["build_contract"]["again_built"] is not False
            or "\n  push:" in workflow_text
            or "python tools\\wf0-factory-census\\build.py dx0-workflow-build" not in workflow_text
            or "configure_and_build(" in dx0_build_body
            or "configure_and_build_host(transaction / \"a\", sdk)" not in dx0_build_body
            or "configure_and_build_host(transaction / \"b\", sdk)" not in dx0_build_body
            or call_surface["audio_processor_method_calls_absent"] is not True
            or audio_method_regression["receiver_independent"] is not True):
        fail("DX0 accepted fixture/host-only producer contract failed")

    current_role = dx0_source_role(current)
    valid = synthetic_valid_result(plan_sha, execution_role=current_role)
    validate_result(valid)
    validate_retained_result(valid)
    malformed_mutations = _malformed_result_mutations()
    parity_rejected: list[str] = []
    for name, mutate in malformed_mutations.items():
        candidate = copy.deepcopy(valid)
        mutate(candidate)
        mac_rejected = _validator_rejects(validate_result, candidate)
        deck_rejected = _validator_rejects(validate_retained_result, candidate)
        if mac_rejected != deck_rejected or not mac_rejected:
            fail(f"DX0 Deck/Mac result-validator parity differs: {name}")
        parity_rejected.append(name)
    with tempfile.TemporaryDirectory(prefix="dx0-validator-parity-") as temporary:
        result_path = pathlib.Path(temporary) / "DX0_TRANSACTION_RESULT.json"
        result_path.write_bytes(canonical_json(valid))
        sidecar = result_path.with_suffix(result_path.suffix + ".sha256")
        sidecar.write_bytes(
            f"{sha256_bytes(result_path.read_bytes())}  {result_path.name}\n".encode()
        )
        if (validate_retained_result_file(result_path) != valid):
            fail("DX0 Deck retained-result-file validator rejected valid input")
        from evidence import validate_result_file as mac_validate_result_file
        if mac_validate_result_file(result_path) != valid:
            fail("DX0 Mac retained-result-file validator rejected valid input")
        sidecar.write_text("0" * 64 + f"  {result_path.name}\n", encoding="utf-8")
        file_rejections = []
        for validator in (validate_retained_result_file, mac_validate_result_file):
            try:
                validator(result_path)
            except RuntimeError:
                file_rejections.append(True)
        if file_rejections != [True, True]:
            fail("DX0 Deck/Mac result-sidecar rejection parity differs")
        result_path.write_text(
            json.dumps(valid, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        sidecar.write_bytes(
            f"{sha256_bytes(result_path.read_bytes())}  {result_path.name}\n".encode()
        )
        canonical_rejections = []
        for validator in (validate_retained_result_file, mac_validate_result_file):
            try:
                validator(result_path)
            except RuntimeError:
                canonical_rejections.append(True)
        if canonical_rejections != [True, True]:
            fail("DX0 Deck/Mac canonical-result rejection parity differs")
    consumer = _role("3")
    admission = result_admission_receipt(valid, consumer_source=consumer, renderer=renderer)
    if (admission["artifact_producer_source"] == admission["deck_execution_source"]
            or admission["deck_execution_source"] == admission["evidence_consumer_source"]
            or admission["fresh_deck_execution_for_consumer"] is not False):
        fail("DX0 distinct P/E/C result admission failed")

    with tempfile.TemporaryDirectory(prefix="dx0-state-test-") as temporary:
        driver = driver_class(source_commit, DX0_PLAN_ID, proof_root=pathlib.Path(temporary) / "transactions")
        nonce = "a" * 32
        recovered = driver._reconcile_run(FakeGitHub(source_commit, nonce), nonce)
        if recovered != 73:
            fail("DX0 lost-dispatch-ack reconciliation failed")
        settling = SettlingRunGitHub(
            source_commit, nonce, dx0_identity_sha256(build)
        )
        settled_run, settled_identity = driver._read_exact_run(
            settling, 73, nonce, attempts=2, delay_seconds=0,
        )
        if (settling.calls != 2 or settled_run["id"] != 73
                or settled_identity["id"] != 73):
            fail("DX0 accepted-run metadata-settling recovery failed")
        clock_value = [0.0]

        def clock() -> float:
            return clock_value[0]

        def sleeper(seconds: float) -> None:
            clock_value[0] += seconds

        never = NeverCompletesGitHub(
            source_commit, nonce, dx0_identity_sha256(build)
        )
        if driver._wait_for_run(
                never, 74, nonce, timeout_seconds=2.0, poll_seconds=1.0,
                clock=clock, sleeper=sleeper) is not None or never.calls != 3:
            fail("DX0 bounded workflow-completion wait proof failed")
        driver.record_run_invocation()
        driver.record_run_invocation()
        if driver.state["run_invocation_count"] != 2:
            fail("DX0 run-invocation ledger proof failed")
        original_derive = copy.deepcopy(driver.state["phases"]["derive_identities"])
        driver.phase("derive_identities", "reused", inputs={"replacement": True},
                     outputs={"replacement": True})
        if driver.state["phases"]["derive_identities"] != original_derive:
            fail("DX0 completed phase provenance was rewritten")
        driver.set_state("source_frozen")
        driver.set_state("planned")
        if driver.state["state"] != "source_frozen":
            fail("DX0 transaction state regressed during resume")
        lock_parent = pathlib.Path(temporary) / "locks"
        lock = driver_class.acquire_single_writer(lock_parent, "key", {"intent": "one"})
        duplicate_rejected = False
        try:
            driver_class.acquire_single_writer(lock_parent, "key", {"intent": "two"})
        except RuntimeError:
            duplicate_rejected = True
        if not duplicate_rejected:
            fail("DX0 duplicate driver single-writer proof failed")
        driver_class.retire_single_writer(
            lock, publication_valid=True, expected_intent={"intent": "one"}
        )

        late_publication = {"identity": "accepted"}
        publication_reads = [None, late_publication]

        def read_late_publication() -> dict[str, str] | None:
            return publication_reads.pop(0)

        guarded_lock, observed_publication, may_start = (
            driver_class.acquire_after_publication_recheck(
                lock_parent, "late-key", {"intent": "same-input"},
                read_late_publication,
            )
        )
        if (guarded_lock is None
                or observed_publication != late_publication
                or may_start is not False
                or not (lock_parent / "late-key").exists()):
            fail("DX0 post-lock publication recheck proof failed")
        driver_class.retire_single_writer(
            guarded_lock, publication_valid=True,
            expected_intent={"intent": "same-input"},
        )

        remote = pathlib.Path(temporary) / "remote-result"
        remote.mkdir()
        remote_result = remote / "DX0_TRANSACTION_RESULT.json"
        remote_result.write_bytes(canonical_json(valid))
        remote_sidecar = remote / "DX0_TRANSACTION_RESULT.json.sha256"
        remote_sidecar.write_bytes(
            f"{sha256_bytes(canonical_json(valid))}  DX0_TRANSACTION_RESULT.json\n".encode()
        )

        class CompletedDeckWithLostAck:
            fetch_count = 0

            @staticmethod
            def run(script: str) -> bytes:
                return b"present"

            def fetch(self, source: str, destination: pathlib.Path,
                      *, timeout: float = 300.0) -> None:
                self.fetch_count += 1
                selected = remote_sidecar if source.endswith(".sha256") else remote_result
                destination.write_bytes(selected.read_bytes())

        deck = CompletedDeckWithLostAck()
        recovered_result = result_recovery(
            deck, valid["execution_input"]["identity_sha256"], plan_sha,
            result_parent=pathlib.Path(temporary) / "recovered-results",
        )
        if (recovered_result is None or recovered_result["result"] != valid
                or deck.fetch_count != 2):
            fail("DX0 completed-Deck lost-ack retrieval proof failed")

        class InterruptedResultTransfer(CompletedDeckWithLostAck):
            def __init__(self, fail_after_sidecar_write: bool) -> None:
                self.fetch_count = 0
                self.fail_after_sidecar_write = fail_after_sidecar_write

            def fetch(self, source: str, destination: pathlib.Path,
                      *, timeout: float = 300.0) -> None:
                self.fetch_count += 1
                selected = remote_sidecar if source.endswith(".sha256") else remote_result
                destination.write_bytes(selected.read_bytes())
                if (self.fetch_count == 1 and not self.fail_after_sidecar_write
                        or self.fetch_count == 2 and self.fail_after_sidecar_write):
                    raise RuntimeError("injected retained-result transfer interruption")

        partial_parent = pathlib.Path(temporary) / "partial-transfer-results"
        partial = InterruptedResultTransfer(False)
        try:
            result_recovery(
                partial, valid["execution_input"]["identity_sha256"], plan_sha,
                result_parent=partial_parent,
            )
        except RuntimeError:
            pass
        else:
            fail("DX0 interrupted result-file transfer was not injected")
        retry_partial = CompletedDeckWithLostAck()
        resumed_partial = result_recovery(
            retry_partial, valid["execution_input"]["identity_sha256"], plan_sha,
            result_parent=partial_parent,
        )
        quarantines = [
            path for path in partial_parent.iterdir()
            if path.name.startswith(
                ".dx0-result-retrieval-"
                + valid["execution_input"]["identity_sha256"] + ".partial-"
            )
        ]
        if (resumed_partial is None or resumed_partial["result"] != valid
                or retry_partial.fetch_count != 2 or len(quarantines) != 1
                or not (quarantines[0] / "DX0_TRANSACTION_RESULT.json").is_file()):
            fail("DX0 partial result-file transfer recovery failed")

        complete_stage_parent = pathlib.Path(temporary) / "complete-stage-results"
        complete_stage = InterruptedResultTransfer(True)
        try:
            result_recovery(
                complete_stage, valid["execution_input"]["identity_sha256"],
                plan_sha, result_parent=complete_stage_parent,
            )
        except RuntimeError:
            pass
        else:
            fail("DX0 interrupted sidecar acknowledgement was not injected")
        recover_without_fetch = CompletedDeckWithLostAck()
        resumed_complete = result_recovery(
            recover_without_fetch, valid["execution_input"]["identity_sha256"],
            plan_sha, result_parent=complete_stage_parent,
        )
        if (resumed_complete is None or resumed_complete["result"] != valid
                or resumed_complete["disposition"] != "recovered_complete_stage"
                or recover_without_fetch.fetch_count != 0):
            fail("DX0 complete retrieval-stage acknowledgement recovery failed")

        symlink_parent = pathlib.Path(temporary) / "symlink-results"
        symlink_parent.mkdir()
        (symlink_parent / valid["execution_input"]["identity_sha256"]).symlink_to(remote)
        symlink_rejected = False
        try:
            result_recovery(
                deck, valid["execution_input"]["identity_sha256"], plan_sha,
                result_parent=symlink_parent,
            )
        except RuntimeError:
            symlink_rejected = True
        if not symlink_rejected:
            fail("DX0 retained-result symlink rejection proof failed")

    synthetic_host = {
        "build_receipt": {
            "windows_build_input": {
                "sha256": valid["host_artifact"]["windows_build_input_sha256"]
            },
            "workflow": {
                "run_id": valid["host_artifact"]["workflow_run_id"],
                "run_attempt": valid["host_artifact"]["run_attempt"],
            },
        },
        "custody": {
            "artifact": {"id": valid["host_artifact"]["artifact_id"]},
            "producer_source": valid["artifact_producer_source"],
        },
        "manifest_sha256": valid["host_artifact"]["manifest_sha256"],
        "build_receipt_sha256": valid["host_artifact"]["build_receipt_sha256"],
        "custody_sha256": valid["host_artifact"]["mac_custody_receipt_sha256"],
    }
    synthetic_fixture = {
        "identity_sha256": valid["accepted_fixture"]["identity_sha256"],
        "identity": {"bundle_manifest": {"records": [{
            "path": "Contents/x86_64-win/again.vst3",
            "sha256": DX0_AGAIN_MODULE_SHA256,
        }]}},
        "receipt_sha256": valid["accepted_fixture"]["mac_store_receipt_sha256"],
    }
    synthetic_handoff = {
        "receipt": {
            "implementation_source": valid["deck_execution_source"],
            "bundle": {
                "sha256": valid["source_handoff"]["bundle_sha256"],
                "advertised_ref": valid["source_handoff"]["advertised_ref"],
            },
        },
        "receipt_sha256": valid["source_handoff"]["receipt_sha256"],
    }

    with tempfile.TemporaryDirectory(prefix="dx0-host-publication-recovery-") as temporary:
        recovery_root = pathlib.Path(temporary)
        host_driver = driver_class(
            source_commit, DX0_PLAN_ID,
            proof_root=recovery_root / "transactions",
        )
        host_driver.set_state("source_frozen")
        host_driver.set_state("fixture_verified")
        cached_host = copy.deepcopy(synthetic_host)
        cached_host["build_receipt"]["windows_build_input"]["sha256"] = (
            host_driver.build_input_sha
        )
        cached_host["custody"]["producer_source"] = current_role
        phase_nonce = "d" * 32
        host_intent = {
            "windows_build_input_sha256": host_driver.build_input_sha,
            "host_mode": "host_only", "source_sha": source_commit,
            "phase_nonce": phase_nonce,
            "operation_nonce": host_driver.state["operation_nonce"],
        }
        host_lock_parent = recovery_root / "host-locks"
        host_lock = driver_class.acquire_single_writer(
            host_lock_parent,
            f"{host_driver.build_input_sha}-host_only",
            host_intent,
        )
        host_driver.phase(
            "reuse_or_produce_host", "in_flight", inputs=host_intent,
            outputs={"run_id": 1}, phase_nonce=phase_nonce,
        )
        original_set_state = host_driver.set_state

        def interrupt_before_lock_retirement(_state: str) -> None:
            raise RuntimeError("injected post-publication recovery interruption")

        host_driver.set_state = interrupt_before_lock_retirement
        try:
            host_driver._retain_cached_host(
                cached_host, lock_parent=host_lock_parent
            )
        except RuntimeError:
            pass
        else:
            fail("DX0 host publication recovery interruption was not injected")
        if (not host_lock.exists()
                or host_driver.state["phases"]["reuse_or_produce_host"]
                    ["disposition"] != "completed"
                or host_driver.state["phases"]["custody_host_artifact"]
                    ["disposition"] != "completed"):
            fail("DX0 host lock retired before durable phase publication")
        host_driver.set_state = original_set_state
        recovered_host = host_driver._retain_cached_host(
            cached_host, lock_parent=host_lock_parent
        )
        if (recovered_host != cached_host or host_lock.exists()
                or host_driver.state["state"] != "host_artifact_verified"
                or host_driver.effect_counts["windows_builds"] != 0):
            fail("DX0 completed host publication recovery started duplicate work")

    store_join_validator(
        valid, synthetic_host, synthetic_fixture,
        verified_handoff=synthetic_handoff,
    )
    original_state = synthetic_original_state(valid)
    observation_state_validator(valid, original_state)
    retained_state = copy.deepcopy(original_state)
    retained_state["state"] = "transaction_result_retained"
    retained_state["phases"].pop("render_and_validate_evidence")
    retained_state["phases"].pop("close_transaction")
    retained_state["effect_counts"]["evidence_renders"] = 0
    retained_projection = observation_state_validator(
        valid, retained_state, require_complete=False
    )
    if (retained_projection.get("ledger_disposition")
            != "result_retained_before_consumer_render"
            or retained_projection.get("phase_count") != 10
            or retained_projection.get("effect_counts", {}).get("evidence_renders") != 0):
        fail("DX0 retained-result evidence-recovery ledger proof failed")
    invalid_retained_state = copy.deepcopy(retained_state)
    invalid_retained_state["effect_counts"]["evidence_renders"] = 1
    try:
        observation_state_validator(
            valid, invalid_retained_state, require_complete=False
        )
    except RuntimeError:
        pass
    else:
        fail("DX0 retained-result evidence-recovery mismatch was accepted")
    ledger_mutations = {
        "missing_phase": lambda state: state["phases"].pop("close_transaction"),
        "wrong_execute_input": lambda state: state["phases"]["execute_deck_batch"]
            ["inputs"].update(deck_execution_input_sha256="0" * 64),
        "wrong_execute_nonce": lambda state: state["phases"]["execute_deck_batch"]
            .update(phase_nonce="0" * 32),
        "wrong_execute_result": lambda state: state["phases"]["execute_deck_batch"]
            ["outputs"].update(retained_result_sha256="0" * 64),
        "reused_execute": lambda state: state["phases"]["execute_deck_batch"]
            .update(disposition="reused"),
        "incomplete_state": lambda state: state.update(state="evidence_rendered"),
    }
    rejected_ledgers: list[str] = []
    for name, mutate in ledger_mutations.items():
        candidate = copy.deepcopy(original_state)
        mutate(candidate)
        if name == "wrong_execute_input":
            receipt = candidate["phases"]["execute_deck_batch"]
            receipt["input_sha256"] = sha256_bytes(canonical_json(receipt["inputs"]))
        try:
            observation_state_validator(valid, candidate)
        except RuntimeError:
            rejected_ledgers.append(name)
    if rejected_ledgers != list(ledger_mutations):
        fail("DX0 original observation ledger-join rejection proof failed")
    mutated_handoff = copy.deepcopy(valid)
    mutated_handoff["source_handoff"]["bundle_sha256"] = "a" * 64
    handoff_mutation_rejected = False
    try:
        store_join_validator(
            mutated_handoff, synthetic_host, synthetic_fixture,
            verified_handoff=synthetic_handoff,
        )
    except RuntimeError:
        handoff_mutation_rejected = True
    if not handoff_mutation_rejected:
        fail("DX0 source-handoff custody mutation proof failed")
    mutated_detached = copy.deepcopy(valid)
    mutated_detached["execution_input"]["detached_worktree_commit"] = "0" * 40
    detached_join_rejected = False
    try:
        store_join_validator(
            mutated_detached, synthetic_host, synthetic_fixture,
            verified_handoff=synthetic_handoff,
        )
    except RuntimeError:
        detached_join_rejected = True
    if not detached_join_rejected:
        fail("DX0 detached-worktree/source-handoff join proof failed")

    plan_rejections = 0
    for mutation in (
        {**plan, "extra": True}, {**plan, "plan_id": "/tmp/plan"},
        {**plan, "live_deck_batch": "sh -c anything"},
        {**plan, "evidence_renderer": {"hook": "run"}},
    ):
        try:
            dx0_validate_plan(mutation)
        except RuntimeError:
            plan_rejections += 1
    if plan_rejections != 4:
        fail("DX0 closed-plan rejection proof failed")

    rejected = []
    for name, mutate in malformed_mutations.items():
        candidate = copy.deepcopy(valid)
        mutate(candidate)
        if _expect_rejected(candidate):
            rejected.append(name)
    if (rejected != list(malformed_mutations)
            or parity_rejected != rejected):
        fail("DX0 incomplete/failed cached-result rejection proof failed")

    mutated_execution_source = copy.deepcopy(valid)
    mutated_execution_source["deck_execution_source"]["tree"] = "0" * 40
    execution_source_mutation_rejected = False
    try:
        store_join_validator(
            mutated_execution_source, synthetic_host, synthetic_fixture,
            verified_handoff=synthetic_handoff,
        )
    except RuntimeError:
        execution_source_mutation_rejected = True
    if not execution_source_mutation_rejected:
        fail("DX0 execution-source identity mutation proof failed")

    with tempfile.TemporaryDirectory(prefix="dx0-production-recovery-") as temporary:
        recovery_root = pathlib.Path(temporary)
        recovery_driver = driver_class(
            source_commit, DX0_PLAN_ID,
            proof_root=recovery_root / "transactions",
        )
        recovered_value = synthetic_valid_result(
            plan_sha, execution_role=current_role
        )
        recovered_value["operation_nonce"] = recovery_driver.state["operation_nonce"]
        execute_nonce = "e" * 32
        recovered_value["original_observation"]["phase_nonce"] = execute_nonce
        execution_sha = recovered_value["execution_input"]["identity_sha256"]
        for state in (
                "source_frozen", "fixture_verified", "host_artifact_verified",
                "handoff_admitted"):
            recovery_driver.set_state(state)
        execute_inputs = {
            "deck_execution_input_sha256": execution_sha,
            "proof_plan_sha256": plan_sha,
            "phase_nonce": execute_nonce,
        }
        recovery_driver.phase(
            "execute_deck_batch", "prepared", inputs=execute_inputs,
            outputs=None, phase_nonce=execute_nonce,
        )
        recovery_driver.effect_counts["deck_executions"] = 1
        recovery_driver.save()
        recovery_driver.set_state("deck_batch_in_flight")
        lock_parent = recovery_root / "execution-locks"
        lock_intent = {
            "deck_execution_input_sha256": execution_sha,
            "proof_plan_sha256": plan_sha,
            "operation_nonce": recovery_driver.state["operation_nonce"],
        }
        lock_key = f"{execution_sha}-{plan_sha}"
        stale_lock = driver_class.acquire_single_writer(
            lock_parent, lock_key, lock_intent
        )
        recovery_calls = [0]

        def recover_completed_publication() -> dict[str, Any]:
            recovery_calls[0] += 1
            return recovered_value

        returned_intent, returned_lock, publication, may_start = (
            execution_writer_recovery(
                recovery_driver, execution_sha, lambda: None,
                recover_completed_publication, lock_parent=lock_parent,
            )
        )
        recovered_handoff = {
            "receipt": {
                "implementation_source": recovered_value["deck_execution_source"],
                "bundle": {
                    "sha256": recovered_value["source_handoff"]["bundle_sha256"],
                    "advertised_ref": recovered_value["source_handoff"]["advertised_ref"],
                },
            },
            "receipt_sha256": recovered_value["source_handoff"]["receipt_sha256"],
        }
        validate_result(publication)
        store_join_validator(
            publication, synthetic_host, synthetic_fixture,
            verified_handoff=recovered_handoff,
        )
        effect_count_before_recovery = recovery_driver.effect_counts["deck_executions"]
        deck_result_recorder(recovery_driver, publication, execution_sha)
        if (returned_intent != lock_intent or returned_lock != stale_lock
                or may_start is not False or recovery_calls != [1]
                or not stale_lock.exists()
                or recovery_driver.state["state"] != "transaction_result_retained"
                or recovery_driver.state["phases"]["execute_deck_batch"]
                    ["disposition"] != "completed"
                or recovery_driver.state["phases"]["execute_deck_batch"]
                    ["inputs"] != execute_inputs
                or recovery_driver.state["phases"]["execute_deck_batch"]
                    ["phase_nonce"] != execute_nonce
                or recovery_driver.effect_counts["deck_executions"]
                    != effect_count_before_recovery):
            fail("DX0 production lost-Deck-ack recovery ordering failed")
        driver_class.retire_single_writer(
            stale_lock, publication_valid=True, expected_intent=lock_intent
        )
        if stale_lock.exists():
            fail("DX0 admitted stale Deck lock retirement failed")

    with tempfile.TemporaryDirectory(prefix="dx0-seed-reuse-") as temporary:
        store = pathlib.Path(temporary) / "verified-store"
        store.mkdir()
        calls = {"verify": 0, "github": 0}

        def verify_existing(selected: pathlib.Path) -> dict[str, Any]:
            calls["verify"] += 1
            if selected != store:
                fail("DX0 fixture reuse selected another store")
            return {"root": str(store), "disposition": "reused"}

        def forbidden_github() -> Any:
            calls["github"] += 1
            fail("DX0 verified fixture reuse contacted GitHub")

        fixture_value, fixture_disposition = seed_fixture_acquirer(
            store=store, verify_store=verify_existing,
            github_factory=forbidden_github,
        )
        if (fixture_value.get("root") != str(store)
                or fixture_disposition != "reused_verified_mac_store"
                or calls != {"verify": 1, "github": 0}):
            fail("DX0 fixture seed reuse-before-download proof failed")
        corrupt_blocked = False

        def reject_corrupt(_selected: pathlib.Path) -> dict[str, Any]:
            fail("synthetic corrupt fixture store")

        try:
            seed_fixture_acquirer(
                store=store, verify_store=reject_corrupt,
                github_factory=forbidden_github,
            )
        except RuntimeError:
            corrupt_blocked = True
        if not corrupt_blocked or calls["github"] != 0:
            fail("DX0 corrupt fixture store fail-closed proof failed")

    with tempfile.TemporaryDirectory(prefix="dx0-handoff-proof-") as handoff_temp:
        handoff_stage = pathlib.Path(handoff_temp) / "source-handoff"
        created_handoff = create_source_handoff(source_commit, handoff_stage)
        verified_real_handoff = verify_source_handoff(handoff_stage, source_commit)
        if (verified_real_handoff["receipt_sha256"]
                != created_handoff["receipt_sha256"]
                or verified_real_handoff["receipt"]["implementation_source"]
                != current_role):
            fail("DX0 self-contained source-handoff verification proof failed")

    with tempfile.TemporaryDirectory(prefix="dx0-render-test-") as temporary:
        output = pathlib.Path(temporary) / "packet"
        rows = [{"row": index, "result": "PASS", "validation": "deterministic",
                 "claim": f"row {index}"} for index in range(1, 15)]
        transaction = {
            "transaction_result": valid, "proof_rows": rows,
            "costs": {"manual_commands": 1, "manually_copied_identifiers": 0,
                      "windows_builds": 0, "artifact_downloads": 0,
                      "custody_operations": 0, "artifact_transfers": 0,
                      "source_transfers": 0, "deck_executions": 0,
                      "evidence_renders": 1, "phase_timings_observed": False,
                      "reused_phases": [], "performed_phases": []},
            "evidence_consumer_source": consumer, "evidence_renderer": renderer,
            "result_admission": admission, "proof_plan": plan,
            "windows_build_input": {"schema": build["schema"],
                                    "sha256": dx0_identity_sha256(build), "record_count": 17},
            "phase_dispositions": {"render": "reused"},
            "fixture_seeding": {"performed_separately": True,
                                "ordinary_run_seeded_fixture": False},
            "invalidation_results": {"renderer_only_external_effects": 0},
        }
        render_packet(output, transaction)
        validate_packet(output)
        render_packet(output, transaction)
        validate_packet(output)
        published = pathlib.Path(temporary) / "published-packet"
        publish_packet(output, published, staging_parent=pathlib.Path(temporary))
        publish_packet(output, published, staging_parent=pathlib.Path(temporary))
        validate_packet(published)
        if any(path.name.startswith(".dx0-evidence-render-")
               for path in pathlib.Path(temporary).iterdir()):
            fail("DX0 atomic evidence staging retirement proof failed")
        if any(path.name.startswith(".dx0-evidence-publish-")
               for path in pathlib.Path(temporary).iterdir()):
            fail("DX0 atomic evidence publication retirement proof failed")

    for boundary in ("manifest", "sidecar"):
        with tempfile.TemporaryDirectory(
                prefix=f"dx0-finalize-{boundary}-") as temporary:
            final_driver = driver_class(
                source_commit, DX0_PLAN_ID,
                proof_root=pathlib.Path(temporary) / "transactions",
            )
            final_result = synthetic_valid_result(
                plan_sha, execution_role=current_role
            )
            final_result["operation_nonce"] = final_driver.state["operation_nonce"]
            final_state = synthetic_original_state(final_result)
            final_state["operation_nonce"] = final_driver.state["operation_nonce"]
            final_state["transaction_key"] = final_driver.transaction_key
            final_state["source"] = final_driver.source_role
            final_state["state"] = "evidence_rendered"
            final_driver.state = final_state
            final_driver.effect_counts = final_state["effect_counts"]
            final_driver.save()
            identity = {
                "schema": DX0_TRANSACTION_SCHEMA,
                "operation_nonce": final_driver.state["operation_nonce"],
                "artifact_producer_source": final_result["artifact_producer_source"],
                "deck_execution_source": final_result["deck_execution_source"],
                "evidence_consumer_source": final_driver.source_role,
                "windows_build_input": {"sha256": final_driver.build_input_sha},
                "host_artifact": final_result["host_artifact"],
                "accepted_fixture": final_result["accepted_fixture"],
                "deck_execution_input": final_result["execution_input"],
                "proof_plan": {"sha256": final_driver.plan_sha},
                "transaction_result": {
                    "sha256": sha256_bytes(canonical_json(final_result))
                },
                "evidence_renderer": {"sha256": final_driver.renderer_sha},
                "phase_receipts": final_driver.state["phases"],
            }
            failed = False
            try:
                final_driver.finalize_transaction(
                    identity, _test_fail_after=boundary
                )
            except RuntimeError:
                failed = True
            manifest = final_driver.root / "DX0_TRANSACTION.json"
            sidecar = final_driver.root / "DX0_TRANSACTION.sha256"
            if (not failed or final_driver.state["state"] != "evidence_rendered"
                    or not manifest.is_file()
                    or (boundary == "manifest" and sidecar.exists())
                    or (boundary == "sidecar" and not sidecar.is_file())):
                fail(f"DX0 {boundary}-boundary finalization fault proof failed")
            final_driver.finalize_transaction(identity)
            retained_identity = json.loads(manifest.read_text(encoding="utf-8"))
            if (final_driver.state["state"] != "transaction_complete"
                    or set(retained_identity) != transaction_keys
                    or sidecar.read_text(encoding="utf-8")
                    != f"{sha256_bytes(manifest.read_bytes())}  DX0_TRANSACTION.json\n"):
                fail(f"DX0 {boundary}-boundary finalization recovery failed")

    invalidation = {
        "historical_build_identity_equal": True,
        "renderer_only": {"build_invalidated": False, "deck_invalidated": False,
                          "renderer_invalidated": True, "external_effects": 0},
        "mac_driver_only": {"build_invalidated": False, "deck_invalidated": False,
                            "external_effects": 0},
        "deck_code": {"build_invalidated": False, "deck_invalidated": True},
        "host_code": {"build_invalidated": True},
        "lost_dispatch_ack_duplicate_builds": 0,
        "lost_deck_ack_duplicate_executions": 0,
        "duplicate_driver_expensive_operations": 0,
        "invalid_cached_results_rejected": rejected,
        "deck_import_closure": {
            "entry_point": "tools/wf0-factory-census/run.py",
            "paths": list(import_closure),
            "all_paths_identity_bound": True,
            "evidence_module_imported": False,
        },
        "result_validator_parity": {
            "valid_result_accepted": True,
            "malformed_cases_rejected": parity_rejected,
            "sidecar_mismatch_rejected": True,
            "noncanonical_json_rejected": True,
        },
    }
    return {"schema": "linux-vst-bridge-dx0-deterministic-proof/v1",
            "proof_row_count": 14, "all_passed": True,
            "invalidation_results": invalidation,
            "external_effects": {"github": 0, "ssh": 0, "proton": 0},
            "synthetic_rows": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 14],
            "live_rows_deferred": [11, 13]}


if __name__ == "__main__":
    raise SystemExit("negative_tests.py is invoked by tools/host-proof.py")


PC0_NATIVE_HARNESS = r'''
#include "component_instance_session.h"
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <limits>
using namespace Steinberg;
using namespace Steinberg::Vst;
using namespace linux_vst_bridge::wf0;
int fault = 0, calls = 0;
#define UNUSED_RESULT(name, args) tresult PLUGIN_API name args override { std::abort(); }
#define UNKNOWN_METHODS \
 UNUSED_RESULT(queryInterface, (const TUID, void**)) \
 uint32 PLUGIN_API addRef() override { std::abort(); } \
 uint32 PLUGIN_API release() override { std::abort(); }
struct Component final : IComponent {
 UNKNOWN_METHODS
 UNUSED_RESULT(initialize, (FUnknown*))
 UNUSED_RESULT(terminate, ())
 UNUSED_RESULT(getControllerClassId, (TUID))
 UNUSED_RESULT(setIoMode, (IoMode))
 UNUSED_RESULT(getRoutingInfo, (RoutingInfo&, RoutingInfo&))
 UNUSED_RESULT(activateBus, (MediaType, BusDirection, int32, TBool))
 UNUSED_RESULT(setActive, (TBool))
 UNUSED_RESULT(setState, (IBStream*))
 UNUSED_RESULT(getState, (IBStream*))
 int32 PLUGIN_API getBusCount(MediaType media, BusDirection direction) override {
   ++calls;
   if (fault == 1) return -1;
   if (fault == 2) return 33;
   if (fault == 3) return 32;
   return media == kEvent && direction == kOutput ? 0 : 1;
 }
 tresult PLUGIN_API getBusInfo(MediaType media, BusDirection direction, int32 index, BusInfo& out) override {
   ++calls;
   const unsigned char* bytes = reinterpret_cast<const unsigned char*>(&out);
   for (std::size_t i = 0; i < sizeof(out); ++i) if (bytes[i] != 0) std::abort();
   if (fault == 4) return kResultFalse;
   if (index != 0) std::abort();
   out.mediaType = fault == 5 ? 99 : media;
   out.direction = fault == 6 ? 99 : direction;
   out.channelCount = fault == 9 ? 0 : (media == kAudio ? 2 : 1);
   out.busType = fault == 10 ? 99 : kMain;
   out.flags = fault == 11 ? 4 : (fault == 12 && media == kEvent ? 3 : 1);
   const char* name = media == kEvent ? "Event In" : direction == kInput ? "Stereo In" : "Stereo Out";
   for (std::size_t i = 0; name[i]; ++i) out.name[i] = name[i];
   if (fault == 7) for (auto& unit : out.name) unit = 'a';
   if (fault == 8) out.name[0] = static_cast<TChar>(0xd800);
   if (fault == 17) out.name[0] = 0;
   if (fault == 18) { out.name[0] = static_cast<TChar>(0xd83d); out.name[1] = static_cast<TChar>(0xde00); out.name[2] = 0; }
   return kResultTrue;
 }
};
struct Audio final : IAudioProcessor {
 UNKNOWN_METHODS
 UNUSED_RESULT(setBusArrangements, (SpeakerArrangement*, int32, SpeakerArrangement*, int32))
 UNUSED_RESULT(setupProcessing, (ProcessSetup&))
 UNUSED_RESULT(setProcessing, (TBool))
 UNUSED_RESULT(process, (ProcessData&))
 uint32 PLUGIN_API getLatencySamples() override { std::abort(); }
 uint32 PLUGIN_API getTailSamples() override { std::abort(); }
 tresult PLUGIN_API getBusArrangement(BusDirection, int32 index, SpeakerArrangement& out) override {
   ++calls;
   if (out != 0 || index != 0) std::abort();
   out = fault == 14 ? 1 : 3;
   return fault == 13 ? kResultFalse : kResultTrue;
 }
 tresult PLUGIN_API canProcessSampleSize(int32 size) override {
   ++calls;
   if (size != kSample32 && size != kSample64) std::abort();
   return fault == 15 ? kResultFalse : fault == 16 ? kInvalidArgument : kResultTrue;
 }
};
int main(int argc, char** argv) {
 if (argc != 3) return 2;
 if (scanner_output_failure_exit(true, 0) != 99 ||
     scanner_output_failure_exit(true, 93) != 93 ||
     scanner_output_failure_exit(true, 97) != 97 ||
     scanner_output_failure_exit(false, 0) != 82) std::abort();
 fault = std::atoi(argv[1]);
 std::size_t overflow = std::numeric_limits<std::size_t>::max();
 if (PreSetupProcessingContractCensus::checked_count(1, overflow)) std::abort();
 Component component; Audio audio;
 EventWriter writer(static_cast<std::size_t>(std::strtoull(argv[2], nullptr, 10)));
 PreSetupProcessingContractCensus census(component, audio, writer);
 int result = 0;
 try { result = census.run(); } catch (int code) { result = code; }
 std::fprintf(stderr, "{\"result\":%d,\"calls\":%d,\"complete\":%s}\n", result, calls, census.contract().empty() ? "false" : "true");
 return 0;
}
'''


def pc0_native_owner_tests(root: pathlib.Path) -> dict[str, Any]:
    """Native deterministic interface doubles exercise the actual C++ borrower.

    This is not a Windows artifact or a plug-in module. No real plug-in is loaded.
    Only the Windows thread/refcount primitives unused by this borrower are shimmed.
    """
    import subprocess
    from common import command_text, SDK_SUBMODULES, sdk_root, write_atomic
    from common import pc0_validate_contract
    from supervise import StreamState, IN_FLIGHT_BLOCKER
    candidates = [sdk_root(), *sorted(pathlib.Path("/private/tmp").glob("pc0-sdk-read.*"))]
    sdk = None
    for candidate in candidates:
        if not (candidate / "pluginterfaces/base/funknown.h").is_file():
            continue
        if all(command_text(["git", "rev-parse", "HEAD"], cwd=candidate / sub) == SDK_SUBMODULES[sub]
               and not command_text(["git", "status", "--porcelain"], cwd=candidate / sub)
               for sub in ("pluginterfaces", "public.sdk")):
            sdk = candidate
            break
    if sdk is None:
        fail("PC0_EVIDENCE_BLOCKED: exact local SDK headers unavailable for owner tests")
    with tempfile.TemporaryDirectory(prefix="pc0-native-owner-") as temporary:
        stage = pathlib.Path(temporary)
        write_atomic(stage / "windows.h", b"#pragma once\n#include <cstdint>\nusing DWORD=std::uint32_t; using LONG=std::int32_t;\ninline DWORD GetCurrentThreadId(){return 1;}\ninline LONG InterlockedIncrement(volatile LONG* p){return __sync_add_and_fetch(p,1);}\ninline LONG InterlockedDecrement(volatile LONG* p){return __sync_sub_and_fetch(p,1);}\n")
        write_atomic(stage / "owner-test.cpp", PC0_NATIVE_HARNESS.encode())
        compile_result = subprocess.run([
            "/usr/bin/clang++", "-std=c++20", "-ffunction-sections", "-fdata-sections",
            "-Wl,-dead_strip", "-I" + str(stage), "-I" + str(sdk),
            "-I" + str(root / "windows-factory-probe/include"),
            "-I" + str(root / "windows-factory-probe/source"),
            str(stage / "owner-test.cpp"),
            str(root / "windows-factory-probe/source/component_instance_session.cpp"),
            "-o", str(stage / "owner-test"),
        ], capture_output=True, timeout=60, check=False)
        if compile_result.returncode:
            fail("PC0 native owner test compilation failed: " + compile_result.stderr.decode()[-4000:])

        def execute(fault: int, cap: int = 1048576) -> tuple[dict[str, Any], list[dict[str, Any]], bytes]:
            returned = subprocess.run([str(stage / "owner-test"), str(fault), str(cap)],
                                      capture_output=True, timeout=5, check=False)
            if returned.returncode:
                fail("PC0 native owner called a prohibited method or failed")
            facts = json.loads(returned.stderr)
            records = [json.loads(line) for line in returned.stdout.splitlines()]
            stream = StreamState()
            for record in records:
                stream.accept(record)
            return facts, records, returned.stdout

        positive, records, raw = execute(0)
        if positive != {"result": 0, "calls": 11, "complete": True}:
            fail("PC0 native positive owner sequence failed")
        contract = pc0_validate_contract(records[-1]["processing_contract"])
        negative_cases = {1: (93, 1), 2: (93, 1), 3: (93, 3), 4: (94, 5),
                          5: (94, 5), 6: (94, 5), 7: (94, 5), 8: (94, 5),
                          9: (94, 5), 10: (94, 5), 11: (94, 5), 12: (94, 7),
                          13: (95, 8), 14: (95, 8), 16: (96, 10)}
        for fault, (code, count) in negative_cases.items():
            facts, _, _ = execute(fault)
            if facts != {"result": code, "calls": count, "complete": False}:
                fail(f"PC0 production owner failure attribution differs: {fault}: {facts}")
        for fault in (15, 17, 18):
            facts, special, _ = execute(fault)
            if facts["result"] != 0:
                fail("PC0 valid false/empty/UTF-16 boundary rejected")
            pc0_validate_contract(special[-1]["processing_contract"], exact_again=False)
        # Actual EventWriter capacity faults, not a second writer model.
        lines = raw.splitlines(keepends=True)
        facts, starts_failed, _ = execute(0, len(lines[0]) - 1)
        if (facts != {"result": 99, "calls": 0, "complete": False}
                or starts_failed
                or any(record.get("state", "").endswith("_in_flight")
                       for record in starts_failed)):
            fail("PC0 failed-start publication falsely attempted a plug-in call")
        facts, completion_failed, _ = execute(0, len(lines[0]))
        if facts != {"result": 99, "calls": 1, "complete": False} or completion_failed[-1]["event"] != "call_started":
            fail("PC0 failed-completion publication did not leave exact unmatched call")
        if any(record.get("event") == "lifecycle"
               and record.get("state", "").endswith("_in_flight") for record in records):
            fail("PC0 emitted an unbound in-flight lifecycle record")
        ordinary, failed_records, failed_raw = execute(1)
        facts, _, _ = execute(1, len(failed_raw) - len(failed_raw.splitlines(keepends=True)[-1]))
        if facts["result"] != 93:
            fail("PC0 output failure erased earlier primary count blocker")
        for operation in ("get_bus_count", "get_bus_info", "get_bus_arrangement", "can_process_sample_size"):
            stream = StreamState()
            for record in records:
                stream.accept(record)
                if record.get("event") == "call_started" and record.get("operation") == operation:
                    break
            if not stream.in_flight or IN_FLIGHT_BLOCKER[operation] not in {
                    "PC0_BUS_COUNT_BLOCKED", "PC0_BUS_INFO_BLOCKED",
                    "PC0_BUS_ARRANGEMENT_BLOCKED", "PC0_SAMPLE_FORMAT_BLOCKED"}:
                fail("PC0 injected unmatched operation attribution failed")
        return {"production_owner_cases": len(negative_cases) + 7,
                "native_not_windows_or_live": True, "contract": contract,
                "positive_records": records, "writer_boundary_passed": True,
                "unmatched_operations": 4}


def _pc0_mutation_identities(root: pathlib.Path, source_commit: str,
                             source_validator: Any,
                             authority_commit: str) -> dict[str, Any]:
    """Actual Git object derivation in a private temporary repository."""
    from common import (PC0_PLAN_ID, command, command_text)
    import os
    import subprocess
    result = {}
    with tempfile.TemporaryDirectory(prefix="pc0-identity-mutations-") as temporary:
        repository = pathlib.Path(temporary) / "repository.git"
        command(["git", "clone", "--bare", "--shared", str(root), str(repository)])
        for label, path in (
            ("original", None), ("renderer", "tools/wf0-factory-census/evidence.py"),
            ("mac", "tools/host-proof.py"), ("deck", "tools/wf0-factory-census/run.py"),
            ("host", "windows-factory-probe/source/component_instance_session.cpp"),
        ):
            commit = source_commit
            if path is not None:
                command(["git", "read-tree", source_commit], cwd=repository)
                original = command(["git", "show", f"{source_commit}:{path}"],
                                   cwd=repository).stdout
                separator = b"" if original.endswith(b"\n") else b"\n"
                mutated = (original + separator
                           + f"# deterministic {label}-only identity mutation\n".encode())
                blob = subprocess.run(["git", "hash-object", "-w", "--stdin"], cwd=repository,
                    input=mutated, capture_output=True, check=True).stdout.decode().strip()
                command(["git", "update-index", "--add", "--cacheinfo", "100644", blob, path], cwd=repository)
                tree = command_text(["git", "write-tree"], cwd=repository)
                environment = {**os.environ,
                    "GIT_AUTHOR_DATE": "2000-01-01T00:00:00Z",
                    "GIT_COMMITTER_DATE": "2000-01-01T00:00:00Z"}
                commit = command_text(["git", "-c", "user.name=PC0 Test",
                    "-c", "user.email=test@example.invalid", "commit-tree", tree,
                    "-p", authority_commit, "-m", "Synthetic PC0 invalidation test"],
                    cwd=repository, env=environment)
            source = None
            source_rejected = False
            try:
                source = source_validator(commit, root=repository)
            except RuntimeError:
                source_rejected = True
            plan = dx0_closed_plan(PC0_PLAN_ID)
            result[label] = {
                "source": source,
                "role": None if source is None else dx0_source_role(source),
                "source_rejected": source_rejected,
                "build": dx0_windows_build_input(commit, root=repository),
                "deck": dx0_deck_execution_input(commit, "a" * 64, "b" * 64,
                    dx0_identity_sha256(plan), root=repository),
                "renderer": dx0_evidence_renderer(commit, root=repository, plan_id=PC0_PLAN_ID),
            }
    return result


def _pc0_materialize_renderer_consumer(root: pathlib.Path, source_commit: str,
                                       parent: pathlib.Path,
                                       source_validator: Any,
                                       authority_commit: str) -> dict[str, Any]:
    """Create one runnable renderer-only C and its isolated exact checkout."""
    from common import (PC0_PLAN_ID, PC0_REF, command,
                        command_text)
    import os
    import subprocess
    repository = parent / "renderer.git"
    checkout = parent / "renderer-consumer"
    command(["git", "clone", "--bare", "--shared", str(root), str(repository)])
    path = "tools/wf0-factory-census/evidence.py"
    command(["git", "read-tree", source_commit], cwd=repository)
    original = command(["git", "show", f"{source_commit}:{path}"], cwd=repository).stdout
    separator = b"" if original.endswith(b"\n") else b"\n"
    mutated = original + separator + b"# deterministic renderer-only identity mutation\n"
    blob = subprocess.run(["git", "hash-object", "-w", "--stdin"], cwd=repository,
        input=mutated, capture_output=True, check=True).stdout.decode().strip()
    command(["git", "update-index", "--add", "--cacheinfo", "100644", blob, path],
            cwd=repository)
    tree = command_text(["git", "write-tree"], cwd=repository)
    environment = {**os.environ,
        "GIT_AUTHOR_DATE": "2000-01-01T00:00:00Z",
        "GIT_COMMITTER_DATE": "2000-01-01T00:00:00Z"}
    commit = command_text(["git", "-c", "user.name=PC0 Test",
        "-c", "user.email=test@example.invalid", "commit-tree", tree,
        "-p", authority_commit, "-m", "Synthetic PC0 renderer-only consumer"],
        cwd=repository, env=environment)
    branch = PC0_REF.removeprefix("refs/heads/")
    command(["git", "update-ref", PC0_REF, commit], cwd=repository)
    command(["git", "update-ref", "refs/heads/pc0-test-original-execution",
             source_commit], cwd=repository)
    command(["git", "symbolic-ref", "HEAD", PC0_REF], cwd=repository)
    command(["git", "clone", "--no-local", str(repository), str(checkout)])
    if (command_text(["git", "rev-parse", "HEAD"], cwd=checkout) != commit
            or command_text(["git", "branch", "--show-current"], cwd=checkout) != branch
            or command_text(["git", "status", "--porcelain=v1", "--untracked-files=all"],
                            cwd=checkout)):
        fail("PC0 synthetic renderer consumer checkout differs")
    source = source_validator(commit, root=checkout)
    return {"checkout": checkout, "source": source, "role": dx0_source_role(source),
            "build": dx0_windows_build_input(commit, root=checkout),
            "deck": dx0_deck_execution_input(commit, "a" * 64, "b" * 64,
                dx0_identity_sha256(dx0_closed_plan(PC0_PLAN_ID)), root=checkout),
            "renderer": dx0_evidence_renderer(commit, root=checkout, plan_id=PC0_PLAN_ID)}


def pc0_synthetic_result(root: pathlib.Path, plan_sha: str, role: dict[str, Any],
                          owner_records: list[dict[str, Any]], contract: dict[str, Any],
                          producer_role: dict[str, Any] | None = None) -> dict[str, Any]:
    from common import PC0_RESULT_SCHEMA, PC0_PLAN_ID, PC0_OPERATIONS
    result = synthetic_valid_result(plan_sha, execution_role=role)
    if producer_role is not None:
        result["artifact_producer_source"] = producer_role
    result["schema"] = PC0_RESULT_SCHEMA
    result["closed_plan"].update(plan_id=PC0_PLAN_ID, expected_result="pc0-pre-setup-contract-complete-v1")
    result["positive_result"].update(audio_processor_method_called=True, processing_contract=contract)
    accepted = json.loads((root /
        "evidence/wa0-windows-vst3-audio-processor-interface-admission/STAGE_TIMELINE.json"
    ).read_bytes())["positive"]
    inherited = [copy.deepcopy(record) for record in accepted
                 if record.get("event") in {"call_started", "call_completed"}]
    if len(inherited) != 44:
        fail("PC0 deterministic accepted WA0 call fixture differs")
    new_calls = [copy.deepcopy(record) for record in owner_records
                 if record.get("event") in {"call_started", "call_completed"}]
    if len(new_calls) != 22:
        fail("PC0 deterministic production-owner call fixture differs")
    ledger: list[dict[str, Any]] = []
    sequence = 0
    for inherited_index in range(22):
        segment = inherited[inherited_index * 2:inherited_index * 2 + 2]
        for record in segment:
            sequence += 1
            record["sequence"] = sequence
            if record["event"] == "call_completed":
                record["attempt_sequence"] = sequence - 1
            ledger.append(record)
        if inherited_index == 13:
            for record in new_calls:
                sequence += 1
                record["sequence"] = sequence
                if record["event"] == "call_completed":
                    record["attempt_sequence"] = sequence - 1
                ledger.append(record)
    result["call_facts"].update(started_count=33, completed_count=33,
        pc0_operation_counts=dict(zip(PC0_OPERATIONS, (4, 3, 2, 2))), ledger=ledger)
    for key in result["integrity"]:
        result["integrity"][key] = sha256_bytes(canonical_json(result[key.removesuffix("_sha256")]))
    return result


def pc0_synthetic_corrective_history(
        result: dict[str, Any], consumer: dict[str, Any],
        current_effects: dict[str, int], *,
        pre_evidence_sha256: str = "1" * 64) -> dict[str, Any]:
    """Build the exact historical-cost object admitted by the V3 renderer."""
    from evidence import (
        PC0_CORRECTIVE_HISTORY_SCHEMA, PC0_FAILED_INPUT_SHA256,
        PC0_FAILED_JOURNAL_SHA256, PC0_FAILED_TRANSACTION,
        PC0_PROOF_PLAN_SHA256, PC0_RECOVERY_SHA256,
        PC0_RUNTIME_DISCOVERY_COMMENT_ID, PC0_V3_APPROVAL_BLOB,
        PC0_V3_AUTHORITY_COMMIT, PC0_V3_AUTHORITY_TREE,
        PC0_V3_DESIGN_BLOB, PC0_V3_DESIGN_SHA256, PC0_V3_REVIEW_ID,
    )
    historical_effects = {
        "windows_builds": 1, "artifact_downloads": 1,
        "custody_operations": 1, "artifact_transfers": 1,
        "source_transfers": 1, "deck_executions": 1,
        "evidence_renders": 0,
    }
    return {
        "schema": PC0_CORRECTIVE_HISTORY_SCHEMA,
        "v2_failed_execution": {
            "transaction_id": PC0_FAILED_TRANSACTION,
            "journal_sha256": PC0_FAILED_JOURNAL_SHA256,
            "source": result["artifact_producer_source"],
            "deck_execution_input_sha256": PC0_FAILED_INPUT_SHA256,
            "proof_plan_sha256": PC0_PROOF_PLAN_SHA256,
            "execute_phase_disposition": "failed",
            "transaction_state": "deck_batch_in_flight",
            "primary_blocker": "PC0_EVIDENCE_BLOCKED",
            "failure_classification": "unresolved_v2_no_diagnostic",
            "local_result_disposition": "absent",
            "remote_preflight": {
                "result": "absent", "result_sidecar": "absent",
                "inner_lock": "present_intent_matched",
                "outer_lock": "present_intent_matched",
            },
            "effect_counts": historical_effects,
            "driver_invocation_count": 1,
        },
        "v2_operator_recovery": {
            "receipt_sha256": PC0_RECOVERY_SHA256,
            "continuation_count": 1,
            "original_driver_invocation_count": 1,
            "original_failure_boundary":
                "mac_execution_reservation_created_before_exact_deck_ssh_discovery",
            "disposition": "continuation_stopped",
            "runtime_discovery_comment_id": PC0_RUNTIME_DISCOVERY_COMMENT_ID,
        },
        "v3_corrective_authority": {
            "merge_commit": PC0_V3_AUTHORITY_COMMIT,
            "merge_tree": PC0_V3_AUTHORITY_TREE,
            "design_blob": PC0_V3_DESIGN_BLOB,
            "design_sha256": PC0_V3_DESIGN_SHA256,
            "review_id": PC0_V3_REVIEW_ID,
            "approval_blob": PC0_V3_APPROVAL_BLOB,
            "prior_journal_sha256": PC0_FAILED_JOURNAL_SHA256,
            "prior_recovery_receipt_sha256": PC0_RECOVERY_SHA256,
            "additional_positive_deck_batches_maximum": 1,
            "additional_windows_builds_maximum": 0,
        },
        "v3_corrective_execution": {
            "transaction_id": result["operation_nonce"],
            "pre_evidence_journal_sha256": pre_evidence_sha256,
            "execution_source": result["deck_execution_source"],
            "evidence_consumer_source": consumer,
            "driver_invocation_count": 1,
            "reservation_count": 1,
            "effect_counts": current_effects,
            "result_sha256": sha256_bytes(canonical_json(result)),
        },
        "cumulative_external_effect_counts": {
            **{key: historical_effects[key] + current_effects[key]
               for key in historical_effects},
            "workflow_dispatches": 1, "again_builds": 0,
            "fixture_seeds": 0, "live_negative_exercises": 0,
        },
        "cumulative_orchestration_counts": {
            "v2_driver_invocations": 1,
            "operator_recovery_continuations": 1,
            "v3_driver_invocations": 1,
            "total_orchestration_entries": 3,
        },
    }


def pc0_deterministic_tests(root: pathlib.Path, source_commit: str, driver_class: Any) -> dict[str, Any]:
    import subprocess
    from common import (PC0_PLAN_ID, PC0_SOURCE_PATHS, PC0_EVIDENCE_PATHS,
                        PC0_OPERATIONS, PC0_PROOF_CLAIMS, write_atomic,
                        pc0_validate_contract)
    from normalize import normalize_pc0_census, validate_wa0_event_order
    from evidence import (
        validate_corrective_preflight,
        validate_failure_diagnostic as validate_mac_failure_diagnostic,
        validate_failure_diagnostic_file as validate_mac_failure_diagnostic_file,
        validate_pre_evidence_snapshot,
        validate_pre_evidence_snapshot_file,
    )
    from run import (
        PC0_DIAGNOSTIC_OPERATIONS, publish_failure_diagnostic,
        validate_failure_diagnostic as validate_deck_failure_diagnostic,
        validate_failure_diagnostic_file as validate_deck_failure_diagnostic_file,
    )
    from unittest.mock import patch
    import sys
    host_module = sys.modules[driver_class.__module__]
    closure = deck_execution_import_closure(root)
    if closure != tuple(DX0_DECK_EXECUTION_PATHS) or "tools/wf0-factory-census/evidence.py" in closure:
        fail("PC0 Deck import closure differs from the seven-record identity")
    source = host_module.pc0_v3_complete_source(source_commit)
    if [item["path"] for item in source["records"]] != list(PC0_SOURCE_PATHS):
        fail("PC0 source envelope differs")
    for path in (*PC0_SOURCE_PATHS,):
        if path.endswith(".py"):
            ast.parse((root / path).read_text(), filename=path)
    scanner_component_call_surface(root)
    workflow = (root / ".github/workflows/wf0-windows-msvc-build.yml").read_text()
    if ("\n  push:" in workflow or "workflow_dispatch:" not in workflow
            or "refs/heads/codex/pc0-windows-vst3-pre-setup-processing-contract" not in workflow
            or "dx0-workflow-build" not in workflow or "persist-credentials: false" not in workflow
            or len(re.findall(r"uses: [^\n]+@[0-9a-f]{40}", workflow)) != 2):
        fail("PC0 workflow closed dispatch structure differs")
    owner = pc0_native_owner_tests(root)
    main_source = (root / "windows-factory-probe/source/main.cpp").read_text()
    component_primary = main_source.find(
        "if (component.primary_exit != 0) primary = component.primary_exit;")
    primary_latched = main_source.find(
        "if (first_primary == 0) first_primary = primary;", component_primary)
    session_published = main_source.find(
        'events.final_lifecycle("component_session_closed"', component_primary)
    exception_owner = main_source.find(
        "return wf0::scanner_output_failure_exit(pc0_mode, first_primary);")
    if min(component_primary, primary_latched, session_published, exception_owner) < 0 or not (
            component_primary < primary_latched < session_published < exception_owner):
        fail("PC0 scanner integration can erase the first primary on output failure")
    normalize_pc0_census(owner["positive_records"], owner["contract"])
    accepted_stream = json.loads((root /
        "evidence/wa0-windows-vst3-audio-processor-interface-admission/STAGE_TIMELINE.json"
    ).read_bytes())["positive"]
    release_position = next(
        index for index, record in enumerate(accepted_stream)
        if record.get("event") == "lifecycle"
        and record.get("state") == "audio_processor_release_in_flight"
    )
    integrated_stream = (
        copy.deepcopy(accepted_stream[:release_position])
        + copy.deepcopy(owner["positive_records"])
        + copy.deepcopy(accepted_stream[release_position:])
    )
    latest_started: int | None = None
    for sequence, record in enumerate(integrated_stream, 1):
        record["sequence"] = sequence
        if record.get("event") == "call_started":
            latest_started = sequence
        elif record.get("event") == "call_completed":
            record["attempt_sequence"] = latest_started
            latest_started = None
    validate_wa0_event_order(integrated_stream, pre_setup=True)
    obsolete_lifecycle = copy.deepcopy(integrated_stream)
    first_pc0 = next(index for index, record in enumerate(obsolete_lifecycle)
                     if record.get("operation") == "get_bus_count")
    obsolete_lifecycle.insert(first_pc0, {
        "event": "lifecycle", "state": "bus_count_in_flight", "sequence": 0,
    })
    if not _validator_rejects(
            lambda value: validate_wa0_event_order(value, pre_setup=True),
            obsolete_lifecycle):
        fail("PC0 normalizer admitted a lifecycle write between durable call boundaries")
    contract_mutations = {
        "duplicate_bus": lambda value: value["buses"].append(copy.deepcopy(value["buses"][0])),
        "reordered_bus": lambda value: value["buses"].__setitem__(slice(0, 2),
            [value["buses"][1], value["buses"][0]]),
        "extra_count_domain": lambda value: value["counts"].append(copy.deepcopy(value["counts"][0])),
        "count_roster_mismatch": lambda value: value["counts"][0].update(count=2),
        "extra_sample_size": lambda value: value["sample_sizes"].append(copy.deepcopy(value["sample_sizes"][0])),
    }
    for label, mutate in contract_mutations.items():
        changed = copy.deepcopy(owner["contract"]); mutate(changed)
        if not _validator_rejects(pc0_validate_contract, changed):
            fail(f"PC0 contract admitted {label}")
    for index in range(11):
        records = copy.deepcopy(owner["positive_records"])
        target = [record for record in records if record["event"] == "call_completed"][index]
        records.remove(target)
        if not _validator_rejects(lambda value: normalize_pc0_census(value, owner["contract"]), records):
            fail("PC0 normalizer admitted a missing completion")
    identities = _pc0_mutation_identities(
        root, source_commit, host_module.pc0_v3_complete_source,
        host_module.PC0_V3_AUTHORITY_COMMIT)
    base = identities["original"]
    for label, expected in (("renderer", (False, False, True)), ("mac", (False, False, False)),
                            ("deck", (False, True, False)), ("host", (True, False, False))):
        observed = tuple(identities[label][key] != base[key] for key in ("build", "deck", "renderer"))
        expected_rejected = label == "host"
        if (observed != expected
                or identities[label]["source_rejected"] is not expected_rejected
                or (not expected_rejected
                    and identities[label]["source"] == base["source"])):
            fail(f"PC0 exact Git invalidation differs for {label}")
    plan = dx0_closed_plan(PC0_PLAN_ID)
    plan_sha = dx0_identity_sha256(plan)
    valid = pc0_synthetic_result(
        root, plan_sha, base["role"], owner["positive_records"],
        owner["contract"], host_module._pc0_failed_source_role())
    validate_result(valid); validate_retained_result(valid)
    malformed = _malformed_result_mutations()
    malformed.update({
        "consumer_in_private": lambda value: value.update(evidence_consumer_source=base["role"]),
        "missing_contract": lambda value: value["positive_result"].pop("processing_contract"),
        "wrong_call_count": lambda value: value["call_facts"].update(started_count=32),
        "unpaired_coordinate": lambda value: value["call_facts"]["ledger"][29].update(direction="kOutput"),
        "incomplete_contract": lambda value: value["positive_result"]["processing_contract"].update(complete=False),
        "extra_latency": lambda value: value["positive_result"]["processing_contract"].update(latency=0),
    })
    for label, mutate in malformed.items():
        value = copy.deepcopy(valid); mutate(value)
        if not all(_validator_rejects(validator, value) for validator in (validate_result, validate_retained_result)):
            fail(f"PC0 strict result validator parity failed: {label}")
    for label, indexes in {"duplicate_call": (29, 27), "reordered_calls": (28, 30)}.items():
        value = copy.deepcopy(valid)
        if label == "duplicate_call":
            value["call_facts"]["ledger"][indexes[0]]["operation"] = \
                value["call_facts"]["ledger"][indexes[1]]["operation"]
        else:
            value["call_facts"]["ledger"][indexes[0]:indexes[1] + 2] = \
                value["call_facts"]["ledger"][indexes[1]:indexes[1] + 2] + \
                value["call_facts"]["ledger"][indexes[0]:indexes[0] + 2]
        if not all(_validator_rejects(validator, value)
                   for validator in (validate_result, validate_retained_result)):
            fail(f"PC0 call ledger admitted {label}")
    from supervise import classify_observed_outcome, pc0_session_is_unclosed
    started = {"event": "call_started", "operation": "get_bus_info", "sequence": 1,
               "interface": "IComponent", "ordinal": None, "tier": None,
               "media_type": "kAudio", "direction": "kInput", "index": 0}
    blocked = {"event": "lifecycle", "state": "pre_setup_census_blocked",
               "primary_blocker": "PC0_BUS_INFO_BLOCKED", "sequence": 2}
    classification_cases = (
        ({"raw_exit": -9, "timed_out": True, "in_flight": started,
          "records": [started]}, "PC0_BUS_INFO_BLOCKED", None),
        ({"raw_exit": -9, "timed_out": True,
          "in_flight": dict(started, operation="release_audio_processor"),
          "records": [started, blocked]}, "PC0_BUS_INFO_BLOCKED", None),
        ({"raw_exit": 99, "timed_out": False, "in_flight": started,
          "records": [started]}, "PC0_CONTRACT_INCOMPLETE", None),
        ({"raw_exit": 99, "timed_out": False, "in_flight": None,
          "records": []}, "PC0_EVIDENCE_BLOCKED", None),
        ({"raw_exit": 0, "timed_out": False, "in_flight": None,
          "records": [], "last_state": "scanner_completed", "cleanup_failed": True},
         "PC0_PROCESS_CLEANUP_BLOCKED", None),
        ({"raw_exit": 94, "timed_out": False, "in_flight": None,
          "records": [blocked], "cleanup_failed": True},
         "PC0_BUS_INFO_BLOCKED", "PC0_PROCESS_CLEANUP_BLOCKED"),
    )
    for arguments, primary, secondary in classification_cases:
        options = {"last_state": None, "gated": True, "hold_gate": False,
                   "cleanup_failed": False, **arguments}
        outcome = classify_observed_outcome(
            mode="pc0-pre-setup-processing-contract", **options)
        if outcome["blocker"] != primary or outcome["secondary_cleanup_blocker"] != secondary:
            fail("PC0 supervisor failure precedence differs")
    if not pc0_session_is_unclosed("pc0-pre-setup-processing-contract", [started], None):
        fail("PC0 unmatched census call did not suppress inherited shutdown")
    # Exercise the real supervision exception/finally boundary. A stream failure
    # followed by failed physical containment must return a typed result; it
    # must not resume the first exception and let run.py retire a live stage.
    import supervise as supervise_module
    from types import SimpleNamespace

    class _Selector:
        def register(self, *args: Any) -> None:
            pass
        def close(self) -> None:
            pass

    fake_root = SimpleNamespace(pid=12345, stdout=object(), stderr=object(),
                                returncode=None)
    fake_environment = SimpleNamespace(
        session=pathlib.Path(tempfile.mkdtemp(prefix="pc0-supervisor-session-")),
        run_id="a" * 32, marker={"fixture": "again"})
    try:
        with patch.object(supervise_module, "verify_environment"), \
                patch.object(supervise_module, "verify_runner_identity",
                             return_value={"identity": "synthetic"}), \
                patch.object(supervise_module, "handshake", return_value=b"binding"), \
                patch.object(supervise_module, "protected_snapshot", return_value={}), \
                patch.object(supervise_module, "command_vector", return_value=["synthetic"]), \
                patch.object(supervise_module, "controlled_environment", return_value={}), \
                patch.object(supervise_module.subprocess, "Popen", return_value=fake_root), \
                patch.object(supervise_module, "process_identity",
                             return_value={"pid": 12345, "start_ticks": 1}), \
                patch.object(supervise_module.selectors, "DefaultSelector", return_value=_Selector()), \
                patch.object(supervise_module, "pump",
                             side_effect=RuntimeError("synthetic stream failure")), \
                patch.object(supervise_module, "cleanup_process",
                             side_effect=RuntimeError("synthetic containment failure")):
            compound = supervise_module.supervise(
                fake_environment, mode="pc0-pre-setup-processing-contract")
        if (compound["blocker"] != "PC0_EVIDENCE_BLOCKED"
                or compound["secondary_cleanup_blocker"] != "PC0_PROCESS_CLEANUP_BLOCKED"
                or compound["cleanup"] != {
                    "owned_descendants_zero": False, "process_group_empty": False}):
            fail("PC0 compound supervision/containment failure was not preserved")
        with patch.object(supervise_module, "verify_environment"), \
                patch.object(supervise_module, "verify_runner_identity",
                             return_value={"identity": "synthetic"}), \
                patch.object(supervise_module, "handshake", return_value=b"binding"), \
                patch.object(supervise_module, "protected_snapshot", return_value={}), \
                patch.object(supervise_module, "command_vector", return_value=["synthetic"]), \
                patch.object(supervise_module, "controlled_environment", return_value={}), \
                patch.object(supervise_module.subprocess, "Popen", return_value=fake_root), \
                patch.object(supervise_module, "process_identity",
                             return_value={"pid": 12345, "start_ticks": 1}), \
                patch.object(supervise_module.selectors, "DefaultSelector", return_value=_Selector()), \
                patch.object(supervise_module, "pump",
                             side_effect=RuntimeError("synthetic stream failure")), \
                patch.object(supervise_module, "cleanup_process", return_value={
                    "owned_descendants_zero": True, "process_group_empty": True}):
            single = supervise_module.supervise(
                fake_environment, mode="pc0-pre-setup-processing-contract")
        if (single["blocker"] != "PC0_EVIDENCE_BLOCKED"
                or single["secondary_cleanup_blocker"] is not None
                or single["cleanup"] != {
                    "owned_descendants_zero": True, "process_group_empty": True}
                or single["inherited_shutdown"]["physical_containment_only"] is not True):
            fail("PC0 supervision failure with successful containment was untyped")

        def semantic_stream_failure(selector: Any, streams: Any, timeout: float) -> None:
            streams.records.append(blocked)
            raise RuntimeError("synthetic stream failure after durable primary")

        with patch.object(supervise_module, "verify_environment"), \
                patch.object(supervise_module, "verify_runner_identity",
                             return_value={"identity": "synthetic"}), \
                patch.object(supervise_module, "handshake", return_value=b"binding"), \
                patch.object(supervise_module, "protected_snapshot", return_value={}), \
                patch.object(supervise_module, "command_vector", return_value=["synthetic"]), \
                patch.object(supervise_module, "controlled_environment", return_value={}), \
                patch.object(supervise_module.subprocess, "Popen", return_value=fake_root), \
                patch.object(supervise_module, "process_identity",
                             return_value={"pid": 12345, "start_ticks": 1}), \
                patch.object(supervise_module.selectors, "DefaultSelector", return_value=_Selector()), \
                patch.object(supervise_module, "pump", side_effect=semantic_stream_failure), \
                patch.object(supervise_module, "cleanup_process", return_value={
                    "owned_descendants_zero": True, "process_group_empty": True}):
            semantic = supervise_module.supervise(
                fake_environment, mode="pc0-pre-setup-processing-contract")
        if (semantic["blocker"] != "PC0_BUS_INFO_BLOCKED"
                or semantic["secondary_cleanup_blocker"] is not None):
            fail("PC0 durable semantic primary was erased by supervision failure")
    finally:
        fake_environment.session.rmdir()
    effects = {"github": 0, "ssh": 0, "proton": 0}
    def forbidden(*args: Any, **kwargs: Any) -> None:
        effects["github"] += 1
        fail("PC0 deterministic test attempted external work")
    with tempfile.TemporaryDirectory(prefix="pc0-planner-test-") as temporary:
        stage = pathlib.Path(temporary)
        driver = driver_class(source_commit, PC0_PLAN_ID, proof_root=stage / "transactions", github_factory=forbidden)
        # Actual single-writer recovery: both missing acknowledgements and duplicate invocation.
        intent = {"input": "a" * 64, "nonce": "b" * 32}
        lock, publication, may_start = driver.acquire_after_publication_recheck(stage / "locks", "phase", intent, lambda: None)
        if not may_start or publication is not None: fail("PC0 cache-miss planner failed")
        recovered = driver.acquire_after_publication_recheck(stage / "locks", "phase", intent, lambda: None, lambda: valid)
        duplicate = driver.acquire_after_publication_recheck(stage / "locks", "phase", intent, lambda: None, lambda: None)
        if recovered[1] != valid or recovered[2] or duplicate[2]: fail("PC0 lost-ack recovery duplicated work")
        # The V3 gate is a predicate-bound one-shot reservation, not a global
        # numeric ceiling. No authority blocks; exact authority admits one;
        # its surviving phase/count blocks a third batch.
        budget_driver = driver_class(
            source_commit, PC0_PLAN_ID,
            proof_root=stage / "corrective-budget", github_factory=forbidden)
        preflight_sha = "5" * 64
        if not _validator_rejects(
                lambda _value: budget_driver.reserve_pc0_corrective(
                    "4" * 64, preflight_sha), {}):
            fail("PC0 corrective reservation admitted without exact authority")
        budget_driver._pc0_corrective_authority_preflight_sha256 = preflight_sha
        reserved = budget_driver.reserve_pc0_corrective("4" * 64, preflight_sha)
        if (reserved["reservation"] != host_module.pc0_corrective_reservation(preflight_sha)
                or budget_driver.effect_counts["deck_executions"] != 1
                or budget_driver.state["phases"]["execute_deck_batch"]["inputs"]
                != reserved["inputs"]):
            fail("PC0 exact corrective reservation was not durable")
        budget_driver._pc0_corrective_authority_preflight_sha256 = preflight_sha
        if not _validator_rejects(
                lambda _value: budget_driver.reserve_pc0_corrective(
                    "4" * 64, preflight_sha), {}):
            fail("PC0 consumed corrective reservation admitted a third batch")
        host_driver_source = (root / "tools/host-proof.py").read_text()
        run_start = host_driver_source.find("def run_transaction")
        handoff_position = host_driver_source.find(
            "_source_handoff_and_admission(driver, ssh)", run_start)
        preflight_position = host_driver_source.find(
            "preflight = _admit_deck_inputs(", handoff_position)
        predicate_position = host_driver_source.find(
            "require_pc0_corrective_authority(", preflight_position)
        lock_position = host_driver_source.find(
            "_reconcile_deck_execution_writer(", predicate_position)
        reservation_position = host_driver_source.find(
            "driver.reserve_pc0_corrective(", lock_position)
        intent_position = host_driver_source.find(
            "result = _run_or_retrieve_deck(", reservation_position)
        if min(handoff_position, preflight_position, predicate_position,
               lock_position, reservation_position, intent_position) < 0 or not (
                handoff_position < preflight_position < predicate_position
                < lock_position < reservation_position < intent_position):
            fail("PC0 preflight/predicate/reservation ordering differs")
        execution_builder = host_driver_source[
            host_driver_source.find("def _run_or_retrieve_deck("):
            host_driver_source.find("def _execute_deck_batch(")]
        if ("PYTHONDONTWRITEBYTECODE=1 /usr/bin/python3 -B "
                not in execution_builder
                or "tools/wf0-factory-census/run.py execute "
                not in execution_builder):
            fail("PC0 corrective execute can dirty its detached source worktree")
        run_source = (root / "tools/wf0-factory-census/run.py").read_text()
        preflight_start = run_source.find("def corrective_preflight(")
        preflight_end = run_source.find(
            "def _pc0_validate_durable_records", preflight_start)
        preflight_body = run_source[preflight_start:preflight_end]
        for prohibited in ("write_atomic(", "create_dx0_environment(",
                           "supervise(", "_publish_result(",
                           "publish_failure_diagnostic("):
            if prohibited in preflight_body:
                fail("PC0 read-only preflight reaches a write/launch path")
        expected_handoff = {
            "receipt_sha256": "a" * 64, "bundle_sha256": "b" * 64,
            "advertised_ref":
                f"refs/handoff/dx0-source/{base['role']['commit']}",
        }
        preflight_receipt = {
            "schema": "linux-vst-bridge-pc0-corrective-deck-preflight/v1",
            "operation_nonce": "4" * 32,
            "execution_source": base["role"],
            "execution_input_sha256": "5" * 64,
            "proof_plan_sha256": plan_sha,
            "source_handoff": expected_handoff,
            "worktree": {
                "commit": base["role"]["commit"],
                "detached": True, "clean": True,
            },
            "process_guard": {
                "process_counts": {key: 0 for key in (
                    "bitwig", "validator", "wine", "proton", "runtime",
                    "umu", "yabridge", "wf0")},
                "prohibited_sibling_count": 0,
            },
            "fixture": {
                "hardware": "Steam Deck Galileo", "os": "SteamOS 3.8.16",
                "architecture": "x86_64", "read_only_mode": "enabled",
                "github_authority_absent": True,
                "forwarded_ssh_agent_absent": True,
            },
            "runner_identity_sha256": RUNNER_DIGEST,
            "protected_snapshot_sha256": "6" * 64,
            "stores": {
                "host_artifact_manifest_sha256":
                    "d0e11c374b7b1cb99b357faaa109e9310edc265c159559bd59a2098148484e9c",
                "producer_run_id": 33812659869,
                "producer_run_attempt": 1, "artifact_id": 9915439437,
                "again_bundle_manifest_sha256":
                    DX0_AGAIN_BUNDLE_MANIFEST_SHA256,
                "accepted_fixture_identity_sha256":
                    "6c87be964d26a7ad06e7a4c69c5c5261d1046e9cfb0b17a225fd24c3e40d0ba6",
                "host_transfer_fallback_selected": False,
                "fixture_transfer_fallback_selected": False,
            },
            "historical_failed_transaction": {
                "transaction_id": host_module.PC0_FAILED_TRANSACTION,
                "execution_input_sha256": host_module.PC0_FAILED_INPUT_SHA256,
                "expected_intent_sha256": host_module.PC0_FAILED_INTENT_SHA256,
                "intent": "present_matched", "result": "absent",
                "result_sidecar": "absent",
                "inner_lock": "present_intent_matched",
                "inner_prepared_intent_sha256":
                    host_module.PC0_FAILED_INTENT_SHA256,
                "outer_lock": "present_intent_matched",
                "outer_prepared_intent_sha256":
                    host_module.PC0_FAILED_INTENT_SHA256,
                "contradictory_pc0_object_count": 0,
            },
            "current_corrective_absence": {
                "execution_intent_absent": True, "reservation_absent": True,
                "mac_execution_lock_absent": True, "outer_lock_absent": True,
                "inner_lock_absent": True, "result_absent": True,
                "result_sidecar_absent": True, "diagnostic_absent": True,
                "diagnostic_sidecar_absent": True,
            },
            "write_effect_counts": {
                "environment_creations": 0,
                "execution_intent_publications": 0,
                "execution_lock_creations": 0, "result_publications": 0,
                "diagnostic_publications": 0,
                "protected_state_mutations": 0,
                "deck_execution_reservations": 0,
                "deck_execution_count_increments": 0, "proton_launches": 0,
            },
        }
        validate_corrective_preflight(
            preflight_receipt, expected_source=base["role"],
            expected_execution_input_sha256="5" * 64,
            expected_plan_sha256=plan_sha,
            expected_operation_nonce="4" * 32,
            expected_handoff=expected_handoff,
        )
        preflight_mutations = {
            "process": lambda value:
                value["process_guard"]["process_counts"].update(wine=1),
            "protected": lambda value:
                value.update(protected_snapshot_sha256="not-a-digest"),
            "runner": lambda value:
                value.update(runner_identity_sha256="0" * 64),
            "store": lambda value:
                value["stores"].update(artifact_id=9915439438),
            "historical_lock": lambda value:
                value["historical_failed_transaction"].update(inner_lock="absent"),
            "current_result": lambda value:
                value["current_corrective_absence"].update(result_absent=False),
            "write": lambda value:
                value["write_effect_counts"].update(proton_launches=1),
        }
        for label, mutate in preflight_mutations.items():
            changed = copy.deepcopy(preflight_receipt)
            mutate(changed)
            if not _validator_rejects(
                    lambda value: validate_corrective_preflight(
                        value, expected_source=base["role"],
                        expected_execution_input_sha256="5" * 64,
                        expected_plan_sha256=plan_sha,
                        expected_operation_nonce="4" * 32,
                        expected_handoff=expected_handoff), changed):
                fail(f"PC0 corrective preflight admitted {label} failure")
        if budget_driver.effect_counts["deck_executions"] != 1:
            fail("PC0 preflight validation mutated corrective budget")

        # Freeze and re-admit one exact canonical transaction-state snapshot at
        # the result-retained boundary, before either render or close exists.
        result_sha = sha256_bytes(canonical_json(valid))
        snapshot_preflight_sha = sha256_bytes(canonical_json(preflight_receipt))
        snapshot_state = synthetic_original_state(valid)
        snapshot_state.update(
            state="transaction_result_retained", source=base["role"],
            plan_sha256=plan_sha, operation_nonce=valid["operation_nonce"],
            run_invocation_count=1,
            effect_counts={
                "windows_builds": 0, "artifact_downloads": 0,
                "custody_operations": 0, "artifact_transfers": 0,
                "source_transfers": 0, "deck_executions": 1,
                "evidence_renders": 0,
            },
        )
        snapshot_state["phases"].pop("render_and_validate_evidence")
        snapshot_state["phases"].pop("close_transaction")
        snapshot_state["phases"]["transfer_and_admit_deck_inputs"] = _phase_receipt(
            "transfer_and_admit_deck_inputs", {}, {
                "corrective_preflight": preflight_receipt,
                "corrective_preflight_sha256": snapshot_preflight_sha,
            }, disposition="reused")
        reservation = host_module.pc0_corrective_reservation(
            snapshot_preflight_sha)
        execute_inputs = {
            "deck_execution_input_sha256":
                valid["execution_input"]["identity_sha256"],
            "proof_plan_sha256": plan_sha,
            "phase_nonce": valid["original_observation"]["phase_nonce"],
            "corrective_reservation": reservation,
        }
        snapshot_state["phases"]["execute_deck_batch"] = _phase_receipt(
            "execute_deck_batch", execute_inputs,
            {"retained_result_sha256": result_sha},
            nonce=valid["original_observation"]["phase_nonce"])
        snapshot_state["phases"]["retrieve_and_retain_result"] = _phase_receipt(
            "retrieve_and_retain_result", {}, {
                "retained_result_sha256": result_sha,
                "result_admission": {
                    "retained_result_sha256": result_sha,
                    "operation_nonce": valid["operation_nonce"],
                    "execution_input_sha256":
                        valid["execution_input"]["identity_sha256"],
                },
            })
        snapshot_arguments = {
            "expected_source": base["role"],
            "expected_operation_nonce": valid["operation_nonce"],
            "expected_execution_input_sha256":
                valid["execution_input"]["identity_sha256"],
            "expected_result_sha256": result_sha,
            "expected_preflight_sha256": snapshot_preflight_sha,
        }
        validate_pre_evidence_snapshot(snapshot_state, **snapshot_arguments)
        for label, mutate in {
                "before_result_admission": lambda value:
                    value["phases"]["retrieve_and_retain_result"]["outputs"].pop(
                        "result_admission"),
                "after_render": lambda value:
                    value["phases"].update(render_and_validate_evidence={}),
                "source_mismatch": lambda value:
                    value.update(source=host_module._pc0_failed_source_role()),
                "effect_mismatch": lambda value:
                    value["effect_counts"].update(deck_executions=0),
                "reservation_mismatch": lambda value:
                    value["phases"]["execute_deck_batch"]["inputs"].update(
                        corrective_reservation={}),
                "result_mismatch": lambda value:
                    value["phases"]["retrieve_and_retain_result"]["outputs"].update(
                        retained_result_sha256="0" * 64),
        }.items():
            changed = copy.deepcopy(snapshot_state)
            mutate(changed)
            if not _validator_rejects(
                    lambda value: validate_pre_evidence_snapshot(
                        value, **snapshot_arguments), changed):
                fail(f"PC0 pre-evidence snapshot admitted {label}")
        snapshot_path = stage / "PC0_CORRECTIVE_PRE_EVIDENCE_STATE.json"
        snapshot_bytes = canonical_json(snapshot_state)
        write_atomic(snapshot_path, snapshot_bytes)
        write_atomic(
            snapshot_path.with_suffix(".json.sha256"),
            f"{sha256_bytes(snapshot_bytes)}  {snapshot_path.name}\n".encode())
        live_path = stage / "DX0_TRANSACTION_STATE.json"
        write_atomic(live_path, snapshot_bytes)
        validate_pre_evidence_snapshot_file(
            snapshot_path, live_journal=live_path,
            require_live_equality=True, **snapshot_arguments)
        late_state = copy.deepcopy(snapshot_state)
        late_state["state"] = "evidence_rendered"
        late_state["effect_counts"]["evidence_renders"] = 1
        late_state["phases"]["render_and_validate_evidence"] = {}
        write_atomic(live_path, canonical_json(late_state))
        validate_pre_evidence_snapshot_file(
            snapshot_path, live_journal=live_path,
            require_live_equality=False, **snapshot_arguments)
        if not _validator_rejects(
                lambda path: validate_pre_evidence_snapshot_file(
                    path, live_journal=live_path,
                    require_live_equality=True, **snapshot_arguments), snapshot_path):
            fail("PC0 final live journal was substituted for the frozen snapshot")
        bad_sidecar = stage / "PC0_BAD_SIDECAR.json"
        write_atomic(bad_sidecar, snapshot_bytes)
        write_atomic(bad_sidecar.with_suffix(".json.sha256"), b"wrong\n")
        if not _validator_rejects(
                lambda path: validate_pre_evidence_snapshot_file(
                    path, require_live_equality=False, **snapshot_arguments),
                bad_sidecar):
            fail("PC0 pre-evidence snapshot sidecar mismatch was admitted")
        noncanonical_snapshot = stage / "PC0_NONCANONICAL.json"
        noncanonical_bytes = json.dumps(snapshot_state, indent=2).encode()
        write_atomic(noncanonical_snapshot, noncanonical_bytes)
        write_atomic(
            noncanonical_snapshot.with_suffix(".json.sha256"),
            f"{sha256_bytes(noncanonical_bytes)}  {noncanonical_snapshot.name}\n".encode())
        if not _validator_rejects(
                lambda path: validate_pre_evidence_snapshot_file(
                    path, require_live_equality=False, **snapshot_arguments),
                noncanonical_snapshot):
            fail("PC0 noncanonical pre-evidence snapshot was admitted")
        supervisor_source = (root / "tools/wf0-factory-census/supervise.py").read_text()
        supervisor_start = supervisor_source.find("def supervise(")
        containment_return = supervisor_source.find("if cleanup_error is not None:", supervisor_start)
        later_validation = supervisor_source.find("if streams.pending:", supervisor_start)
        if containment_return < 0 or later_validation < 0 or containment_return > later_validation:
            fail("PC0 cleanup failure is not returned before later validation")
        adapter = object.__new__(host_module.SSHAdapter)
        adapter.destination = "deck@synthetic.invalid"
        from common import PC0_BLOCKED_OUTCOMES
        if PC0_BLOCKED_OUTCOMES != frozenset({
                "PC0_DESIGN_PREFLIGHT_BLOCKED", "PC0_DESIGN_SCOPE_BLOCKED",
                "PC0_BUS_COUNT_BLOCKED", "PC0_BUS_INFO_BLOCKED",
                "PC0_BUS_ARRANGEMENT_BLOCKED", "PC0_SAMPLE_FORMAT_BLOCKED",
                "PC0_CONTRACT_INCOMPLETE", "PC0_PROCESS_CLEANUP_BLOCKED",
                "PC0_EVIDENCE_BLOCKED", "RETURN_TO_DESIGN_GATE"}):
            fail("PC0 remote blocked taxonomy differs")
        from run import pc0_compose_failure
        primary = pc0_compose_failure(
            RuntimeError("PC0_BUS_INFO_BLOCKED: synthetic semantic failure"),
            retirement_failed=True,
        )
        if (primary is None
                or str(primary) != (
                    "PC0_BUS_INFO_BLOCKED: positive batch did not complete; "
                    "secondary=PC0_PROCESS_CLEANUP_BLOCKED")):
            fail("PC0 environment-retirement failure erased the semantic primary")
        cleanup_only = pc0_compose_failure(None, retirement_failed=True)
        if (cleanup_only is None
                or not str(cleanup_only).startswith("PC0_PROCESS_CLEANUP_BLOCKED:")):
            fail("PC0 environment-retirement failure lacks its exact owner")
        evidence_only = pc0_compose_failure(RuntimeError("synthetic untyped failure"))
        if (evidence_only is None
                or not str(evidence_only).startswith("PC0_EVIDENCE_BLOCKED:")):
            fail("PC0 untyped Deck failure was not fail-closed")
        remote_failure = subprocess.CompletedProcess(
            args=[], returncode=94, stdout=b"",
            stderr=b"DX0_ERROR: PC0_BUS_INFO_BLOCKED: synthetic failure\n")
        with patch.object(host_module.subprocess, "run", return_value=remote_failure):
            try:
                adapter.run("synthetic")
            except host_module.RemoteDeckBlocked as error:
                if error.blocker != "PC0_BUS_INFO_BLOCKED":
                    fail("PC0 remote blocker identity changed")
            else:
                fail("PC0 exact remote blocker was collapsed into handoff failure")
        # The private diagnostic is published atomically inside the retained
        # execution lock and both independent validators admit the same bytes.
        diagnostic_intent = {
            "operation_nonce": "9" * 32, "phase_nonce": "7" * 32,
            "execution_source": base["role"],
            "deck_execution_input_sha256": "8" * 64,
            "proof_plan_sha256": plan_sha,
        }
        zero_call_counts = {name: 0 for name in PC0_DIAGNOSTIC_OPERATIONS}

        def diagnostic_run(raw_exit: int | None, classification: str) -> dict[str, Any]:
            return {
                "records": [], "run_id": "6" * 32, "raw_exit": raw_exit,
                "classification": classification,
                "blocker": "PC0_EVIDENCE_BLOCKED",
                "secondary_cleanup_blocker": None,
                "last_lifecycle": None, "last_in_flight_operation": None,
                "call_counts": zero_call_counts,
                "audio_processor_observer_state":
                    "audio_processor_ownership_unknown",
                "audio_interface_quiescence": False,
                "inherited_shutdown": {
                    "operations": {}, "clean_in_process_shutdown": False,
                    "physical_containment_only": True,
                },
                "cleanup": {
                    "owned_descendants_zero": True,
                    "process_group_empty": True,
                },
                "stdout_sha256": sha256_bytes(b""),
                "stderr_sha256": sha256_bytes(b""), "stderr_bytes": 0,
                "protected_snapshot": {},
                "runner_identity": {
                    "launch_critical_manifest_sha256": RUNNER_DIGEST,
                },
            }

        diagnostics: dict[str, dict[str, Any]] = {}
        for label, raw_exit, classification in (
                ("publication", 99, "output_publication_failed"),
                ("supervision", None, "supervision_failed"),
                ("supervision-observed-exit-99", 99, "supervision_failed")):
            lock = stage / f"diagnostic-{label}"
            lock.mkdir()
            write_atomic(lock / "prepared-intent.json", canonical_json(diagnostic_intent))
            retained = publish_failure_diagnostic(
                lock, diagnostic_run(raw_exit, classification),
                intent=diagnostic_intent, source_role=base["role"],
                retirement_disposition="retired",
            )
            validators = (
                validate_deck_failure_diagnostic_file,
                validate_mac_failure_diagnostic_file,
            )
            for validator in validators:
                accepted = validator(
                    retained["path"], expected_source=base["role"],
                    expected_execution_input_sha256="8" * 64,
                    expected_plan_sha256=plan_sha,
                    expected_operation_nonce="9" * 32,
                    expected_phase_nonce="7" * 32,
                )
                if (accepted["raw_exit"] != raw_exit
                        or accepted["classification"] != classification):
                    fail("PC0 failure diagnostic class/raw-exit join differs")
            diagnostics[label] = retained

        diagnostic_value_validators = (
            validate_deck_failure_diagnostic,
            validate_mac_failure_diagnostic,
        )

        def validate_diagnostic_value(validator: Any,
                                      value: dict[str, Any]) -> dict[str, Any]:
            return validator(
                value, expected_source=base["role"],
                expected_execution_input_sha256="8" * 64,
                expected_plan_sha256=plan_sha,
                expected_operation_nonce="9" * 32,
                expected_phase_nonce="7" * 32,
            )

        lifecycle_diagnostic = copy.deepcopy(diagnostics["supervision"]["diagnostic"])
        lifecycle_diagnostic.update(
            durable_record_count=1,
            durable_records=[{
                "event": "lifecycle", "sequence": 1, "state": "scanner_started",
            }],
            last_lifecycle="scanner_started",
        )
        for validator in diagnostic_value_validators:
            validate_diagnostic_value(validator, lifecycle_diagnostic)

        completed_call_diagnostic = copy.deepcopy(
            diagnostics["supervision"]["diagnostic"])
        completed_call_diagnostic["durable_records"] = [
            {
                "event": "call_started", "sequence": 1,
                "operation": "get_bus_count", "interface": "IComponent",
                "ordinal": None, "tier": None,
                "media_type": "kAudio", "direction": "kInput",
            },
            {
                "event": "call_completed", "sequence": 2,
                "attempt_sequence": 1, "operation": "get_bus_count",
                "interface": "IComponent", "ordinal": None, "tier": None,
                "return_kind": "int32", "i32_result": 1,
                "media_type": "kAudio", "direction": "kInput",
            },
        ]
        completed_call_diagnostic["durable_record_count"] = 2
        completed_call_diagnostic["call_counts"] = {
            **zero_call_counts, "get_bus_count": 1,
        }
        for validator in diagnostic_value_validators:
            validate_diagnostic_value(validator, completed_call_diagnostic)

        diagnostic_record_faults: dict[str, dict[str, Any]] = {}
        unknown_lifecycle = copy.deepcopy(lifecycle_diagnostic)
        unknown_lifecycle["durable_records"][0]["state"] = "invented_lifecycle"
        unknown_lifecycle["last_lifecycle"] = "invented_lifecycle"
        diagnostic_record_faults["unknown_lifecycle"] = unknown_lifecycle
        nonmonotonic_lifecycle = copy.deepcopy(lifecycle_diagnostic)
        nonmonotonic_lifecycle["durable_records"] = [
            {"event": "lifecycle", "sequence": 1, "state": "module_opened"},
            {"event": "lifecycle", "sequence": 2, "state": "readiness_announced"},
        ]
        nonmonotonic_lifecycle["durable_record_count"] = 2
        nonmonotonic_lifecycle["last_lifecycle"] = "readiness_announced"
        diagnostic_record_faults["nonmonotonic_lifecycle"] = nonmonotonic_lifecycle
        sequence_gap = copy.deepcopy(lifecycle_diagnostic)
        sequence_gap["durable_records"][0]["sequence"] = 2
        diagnostic_record_faults["sequence_gap"] = sequence_gap
        missing_start_field = copy.deepcopy(completed_call_diagnostic)
        missing_start_field["durable_records"][0].pop("tier")
        diagnostic_record_faults["missing_start_field"] = missing_start_field
        unknown_interface = copy.deepcopy(completed_call_diagnostic)
        unknown_interface["durable_records"][0]["interface"] = "IUnknown"
        unknown_interface["durable_records"][1]["interface"] = "IUnknown"
        diagnostic_record_faults["unknown_interface"] = unknown_interface
        mutable_coordinate = copy.deepcopy(completed_call_diagnostic)
        mutable_coordinate["durable_records"][1]["direction"] = "kOutput"
        diagnostic_record_faults["mutable_coordinate"] = mutable_coordinate
        unknown_return_kind = copy.deepcopy(completed_call_diagnostic)
        unknown_return_kind["durable_records"][1]["return_kind"] = "invented"
        diagnostic_record_faults["unknown_return_kind"] = unknown_return_kind
        oversized_stderr = copy.deepcopy(lifecycle_diagnostic)
        oversized_stderr["stderr_bytes"] = 65537
        diagnostic_record_faults["oversized_stderr"] = oversized_stderr
        false_output_publication = copy.deepcopy(lifecycle_diagnostic)
        false_output_publication.update(
            classification="output_publication_failed", raw_exit=None)
        diagnostic_record_faults["false_output_publication"] = false_output_publication
        extra_top_level_key = copy.deepcopy(lifecycle_diagnostic)
        extra_top_level_key["unreviewed"] = True
        diagnostic_record_faults["extra_top_level_key"] = extra_top_level_key
        missing_top_level_key = copy.deepcopy(lifecycle_diagnostic)
        missing_top_level_key.pop("run_id")
        diagnostic_record_faults["missing_top_level_key"] = missing_top_level_key
        for label, faulty in diagnostic_record_faults.items():
            for validator in diagnostic_value_validators:
                if not _validator_rejects(
                        lambda value, validator=validator:
                            validate_diagnostic_value(validator, value), faulty):
                    fail(f"PC0 {label} diagnostic mutation was admitted")
        publication_path = diagnostics["publication"]["path"]
        if not all(_validator_rejects(
                lambda value, validator=validator: validator(
                    value, expected_source=base["role"],
                    expected_execution_input_sha256="0" * 64,
                    expected_plan_sha256=plan_sha,
                    expected_operation_nonce="9" * 32,
                    expected_phase_nonce="7" * 32), publication_path)
                for validator in (
                    validate_deck_failure_diagnostic_file,
                    validate_mac_failure_diagnostic_file)):
            fail("PC0 mismatched diagnostic join was admitted")
        duplicate_data = publication_path.read_bytes().replace(
            b'"schema":', b'"schema":"attacker/v1","schema":', 1)
        for label, data, sidecar in (
                ("noncanonical",
                 json.dumps(diagnostics["publication"]["diagnostic"],
                            indent=2, sort_keys=True).encode(), None),
                ("duplicate-key", duplicate_data, None),
                ("sidecar", publication_path.read_bytes(), b"wrong\n")):
            tampered = stage / f"diagnostic-{label}.json"
            write_atomic(tampered, data)
            write_atomic(
                tampered.with_suffix(".json.sha256"),
                sidecar or f"{sha256_bytes(data)}  {tampered.name}\n".encode())
            for validator in (
                    validate_deck_failure_diagnostic_file,
                    validate_mac_failure_diagnostic_file):
                if not _validator_rejects(
                        lambda value, validator=validator: validator(
                            value, expected_source=base["role"],
                            expected_execution_input_sha256="8" * 64,
                            expected_plan_sha256=plan_sha,
                            expected_operation_nonce="9" * 32,
                            expected_phase_nonce="7" * 32), tampered):
                    fail(f"PC0 {label} diagnostic was admitted")
        diagnostic_symlink = stage / "diagnostic-symlink.json"
        diagnostic_symlink.symlink_to(publication_path)
        write_atomic(
            diagnostic_symlink.with_suffix(".json.sha256"),
            f"{sha256_bytes(publication_path.read_bytes())}  "
            f"{diagnostic_symlink.name}\n".encode(),
        )
        for validator in (
                validate_deck_failure_diagnostic_file,
                validate_mac_failure_diagnostic_file):
            if not _validator_rejects(
                    lambda value, validator=validator: validator(
                        value, expected_source=base["role"],
                        expected_execution_input_sha256="8" * 64,
                        expected_plan_sha256=plan_sha,
                        expected_operation_nonce="9" * 32,
                        expected_phase_nonce="7" * 32), diagnostic_symlink):
                fail("PC0 symlink diagnostic was admitted")

        recovery_driver = driver_class(
            source_commit, PC0_PLAN_ID, proof_root=stage / "known-blocker-recovery",
            github_factory=forbidden,
        )
        recovery_nonce = "7" * 32
        recovery_inputs = {
            "deck_execution_input_sha256": "8" * 64,
            "proof_plan_sha256": recovery_driver.plan_sha,
            "phase_nonce": recovery_nonce,
        }
        recovery_driver.phase(
            "execute_deck_batch", "prepared", inputs=recovery_inputs,
            outputs=None, phase_nonce=recovery_nonce,
        )
        retained_diagnostic = diagnostics["publication"]
        diagnostic_custody = {
            "diagnostic": retained_diagnostic["diagnostic"],
            "sha256": retained_diagnostic["sha256"],
            "disposition": "retained",
        }
        try:
            host_module._recover_after_known_deck_blocker(
                recovery_driver, "PC0_EVIDENCE_BLOCKED", recovery_inputs,
                recovery_nonce, lambda: None, lambda value: value,
                lambda: diagnostic_custody,
            )
        except RuntimeError as error:
            if str(error) != "PC0_EVIDENCE_BLOCKED":
                fail("PC0 known remote blocker was erased during diagnostic admission")
        else:
            fail("PC0 known remote blocker recovery did not fail closed")
        expected_failure_outputs = {
            "outward_blocker": "PC0_EVIDENCE_BLOCKED",
            "primary_blocker": "PC0_EVIDENCE_BLOCKED",
            "secondary_cleanup_blocker": None,
            "classification": "output_publication_failed",
            "failure_diagnostic_sha256": retained_diagnostic["sha256"],
            "custody_disposition": "retained",
        }
        if recovery_driver.state["phases"]["execute_deck_batch"] != {
                "phase": "execute_deck_batch", "phase_nonce": recovery_nonce,
                "disposition": "failed",
                "input_sha256": sha256_bytes(canonical_json(recovery_inputs)),
                "inputs": recovery_inputs, "outputs": expected_failure_outputs,
        }:
            fail("PC0 failed recovery did not retain the exact diagnostic primary")
        missing_driver = driver_class(
            source_commit, PC0_PLAN_ID, proof_root=stage / "missing-diagnostic",
            github_factory=forbidden,
        )
        missing_driver.phase(
            "execute_deck_batch", "prepared", inputs=recovery_inputs,
            outputs=None, phase_nonce=recovery_nonce,
        )
        if not _validator_rejects(
                lambda _value: host_module._recover_after_known_deck_blocker(
                    missing_driver, "PC0_EVIDENCE_BLOCKED", recovery_inputs,
                    recovery_nonce, lambda: None, lambda value: value,
                    lambda: None), {}):
            fail("PC0 missing failure diagnostic authorized recovery")
        unknown_driver = driver_class(
            source_commit, PC0_PLAN_ID, proof_root=stage / "unknown-outcome-recovery",
            github_factory=forbidden,
        )
        unknown_nonce = "7" * 32
        unknown_inputs = {
            "deck_execution_input_sha256": "6" * 64,
            "proof_plan_sha256": unknown_driver.plan_sha,
            "phase_nonce": unknown_nonce,
        }
        unknown_driver.phase(
            "execute_deck_batch", "prepared", inputs=unknown_inputs,
            outputs=None, phase_nonce=unknown_nonce,
        )
        recovery_reads = 0

        def one_successful_recovery() -> dict[str, Any] | None:
            nonlocal recovery_reads
            recovery_reads += 1
            if recovery_reads > 1:
                raise RuntimeError("synthetic second recovery transport failure")
            return {"result": valid}

        admitted = host_module._recover_after_unknown_deck_outcome(
            unknown_driver, unknown_inputs, unknown_nonce,
            one_successful_recovery, lambda value: value["result"], lambda: None,
        )
        if admitted != valid or recovery_reads != 1:
            fail("PC0 lost acknowledgement performed a duplicate recovery read")
        path = stage / "DX0_TRANSACTION_RESULT.json"
        write_atomic(path, canonical_json(valid)); write_atomic(path.with_suffix(".json.sha256"),
            f"{sha256_bytes(canonical_json(valid))}  {path.name}\n".encode())
        from evidence import validate_result_file
        validate_result_file(path); validate_retained_result_file(path)
        write_atomic(path.with_suffix(".json.sha256"), b"wrong\n")
        for validator in (validate_result_file, validate_retained_result_file):
            if not _validator_rejects(validator, path): fail("PC0 sidecar mismatch admitted")
        renderer = base["renderer"]
        counts = {key: 0 for key in ("windows_builds", "artifact_downloads", "custody_operations",
                                     "artifact_transfers", "source_transfers", "deck_executions")}
        counts["deck_executions"] = 1
        current_effects = {**counts, "evidence_renders": 1}
        invalidation = {
            "synthetic_git_mutations": True, "renderer_only_external_effects": 0,
            "deck_import_closure": list(closure), "result_validator_parity_cases": len(malformed),
            "production_owner_cases": owner["production_owner_cases"],
            "missing_completions_rejected": 11, "writer_boundary_passed": True,
            "renderer_only": {"build": False, "deck": False, "renderer": True},
            "mac_only": {"build": False, "deck": False},
            "deck_only": {"build": False, "deck": True}, "host_only": {"build": True},
            "lost_ack_and_duplicate_work": 0,
            "corrective_history": pc0_synthetic_corrective_history(
                valid, base["role"], current_effects),
        }
        proof_rows = [
            {"row": row, "result": "PASS",
             "validation": "deterministic" if row in {1, 2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 15} else "live",
             "claim": PC0_PROOF_CLAIMS[row - 1]}
            for row in range(1, 17)
        ]
        transaction = {
            "transaction_result": valid, "evidence_consumer_source": base["role"],
            "evidence_renderer": renderer,
            "result_admission": result_admission_receipt(valid, consumer_source=base["role"], renderer=renderer),
            "windows_build_input": {"schema": base["build"]["schema"],
                                    "sha256": valid["host_artifact"]["windows_build_input_sha256"], "record_count": 17},
            "proof_rows": proof_rows,
            "costs": {**current_effects, "manual_commands": 1,
                      "manually_copied_identifiers": 0},
            "phase_dispositions": {
                "derive_identities": "completed", "plan_external_work": "completed",
                "freeze_source": "completed", "verify_fixture": "reused",
                "reuse_or_produce_host": "reused", "custody_host_artifact": "reused",
                "create_source_handoff": "reused",
                "transfer_and_admit_deck_inputs": "reused",
                "execute_deck_batch": "completed",
                "retrieve_and_retain_result": "completed",
                "render_and_validate_evidence": "completed", "close_transaction": "completed",
            },
            "invalidation_results": invalidation,
            "renderer_only_reuse": {
                **{key: 0 for key in counts}, "evidence_renders": 1,
                "actual_retained_observation_reused": True,
                "consumer_mutation": "synthetic_renderer_only_Git_revision",
                "observation_source": valid["deck_execution_source"]["commit"]},
        }
        render_packet(stage / "packet", transaction)
        validate_packet(stage / "packet")
        packet_path = stage / "packet/TRANSACTION.json"
        packet = json.loads(packet_path.read_bytes())
        packet["admitted_private_result_sha256"] = "0" * 64
        packet["result_admission"]["retained_result_sha256"] = "0" * 64
        packet["integrity"]["admitted_private_result_sha256"] = "0" * 64
        if not _validator_rejects(
                lambda value: validate_pc0_packet_value(value,
                    json.loads((stage / "packet/COST_AND_INVALIDATION.json").read_bytes())),
                packet):
            fail("PC0 packet admitted a substituted private-result digest")
        accepted_packet = json.loads(packet_path.read_bytes())
        accepted_cost = json.loads((stage / "packet/COST_AND_INVALIDATION.json").read_bytes())
        def corrupt_renderer_extra(packet: dict[str, Any], cost: dict[str, Any]) -> None:
            packet["renderer"]["extra"] = "not-authorized"
            packet["integrity"]["renderer_sha256"] = sha256_bytes(
                canonical_json(packet["renderer"])
            )

        def corrupt_renderer_blob(packet: dict[str, Any], cost: dict[str, Any]) -> None:
            packet["renderer"]["records"][0]["git_blob"] = "f" * 40
            packet["integrity"]["renderer_sha256"] = sha256_bytes(
                canonical_json(packet["renderer"])
            )

        def substitute_final_journal(packet: dict[str, Any], cost: dict[str, Any]) -> None:
            execution = cost["invalidation_cases"]["corrective_history"][
                "v3_corrective_execution"]
            digest = execution.pop("pre_evidence_journal_sha256")
            execution["journal_sha256"] = digest

        def hide_failed_deck_execution(packet: dict[str, Any],
                                       cost: dict[str, Any]) -> None:
            history = cost["invalidation_cases"]["corrective_history"]
            history["v2_failed_execution"]["effect_counts"]["deck_executions"] = 0
            history["cumulative_external_effect_counts"]["deck_executions"] = 1

        def hide_driver_invocation(packet: dict[str, Any],
                                   cost: dict[str, Any]) -> None:
            history = cost["invalidation_cases"]["corrective_history"]
            history["v2_failed_execution"]["driver_invocation_count"] = 0
            history["cumulative_orchestration_counts"].update(
                v2_driver_invocations=0, total_orchestration_entries=2)

        packet_faults = {
            "proof_row_stub": lambda packet, cost: packet["proof_rows"][0].pop("claim"),
            "empty_phase_dispositions": lambda packet, cost: cost.update(phase_dispositions={}),
            "empty_invalidation_cases": lambda packet, cost: cost.update(invalidation_cases={}),
            "wrong_build_input_schema": lambda packet, cost:
                packet["windows_build_input"].update(schema="attacker/v1"),
            "renderer_extra_key_with_rehashed_projection": corrupt_renderer_extra,
            "renderer_wrong_blob_with_rehashed_projection": corrupt_renderer_blob,
            "unbounded_artifact_download_count": lambda packet, cost:
                cost["external_effect_counts"].update(artifact_downloads=999),
            "completed_transfer_relabelled_reused": lambda packet, cost:
                cost["phase_dispositions"].update(
                    create_source_handoff=(
                        "reused" if cost["phase_dispositions"]["create_source_handoff"]
                        == "completed" else "completed")),
            "renderer_reuse_without_render": lambda packet, cost:
                cost["renderer_only_reuse"].update(evidence_renders=0),
            "final_live_journal_substituted": substitute_final_journal,
            "historical_failed_deck_hidden": hide_failed_deck_execution,
            "historical_driver_invocation_hidden": hide_driver_invocation,
        }
        for label, mutate in packet_faults.items():
            changed_packet, changed_cost = copy.deepcopy(accepted_packet), copy.deepcopy(accepted_cost)
            mutate(changed_packet, changed_cost)
            if not _validator_rejects(
                    lambda values: validate_pc0_packet_value(values[0], values[1]),
                    (changed_packet, changed_cost)):
                fail(f"PC0 packet admitted {label}")
        # Exercise the production renderer-reuse helper before any live result
        # exists.  The injected stores retain the same exact schemas and joins;
        # the helper must materialize and execute the actual renderer-only C.
        reuse_result = copy.deepcopy(valid)
        fixture_identity = accepted_fixture_identity()
        fixture_sha = accepted_fixture_identity_sha256()
        fixture = {"identity": fixture_identity, "identity_sha256": fixture_sha,
                   "receipt_sha256": fixture_sha, "disposition": "reused"}
        build_sha = dx0_identity_sha256(base["build"])
        reuse_result["host_artifact"]["windows_build_input_sha256"] = build_sha
        reuse_result["accepted_fixture"] = {
            "identity_sha256": fixture_sha,
            "bundle_manifest_sha256": DX0_AGAIN_BUNDLE_MANIFEST_SHA256,
            "module_sha256": DX0_AGAIN_MODULE_SHA256,
            "mac_store_receipt_sha256": fixture_sha,
            "deck_store_receipt_sha256": fixture_sha,
        }
        execution_input = dx0_deck_execution_input(
            source_commit, reuse_result["host_artifact"]["manifest_sha256"],
            fixture_sha, plan_sha)
        reuse_result["execution_input"]["identity_sha256"] = dx0_identity_sha256(
            execution_input)
        reuse_result["execution_input"]["accepted_fixture_identity_sha256"] = fixture_sha
        host = {
            "build_receipt": {
                "windows_build_input": {"sha256": build_sha},
                "workflow": {"run_id": reuse_result["host_artifact"]["workflow_run_id"],
                             "run_attempt": reuse_result["host_artifact"]["run_attempt"]},
            },
            "custody": {
                "artifact": {"id": reuse_result["host_artifact"]["artifact_id"]},
                "producer_source": reuse_result["artifact_producer_source"],
            },
            "manifest_sha256": reuse_result["host_artifact"]["manifest_sha256"],
            "build_receipt_sha256": reuse_result["host_artifact"]["build_receipt_sha256"],
            "custody_sha256": reuse_result["host_artifact"]["mac_custody_receipt_sha256"],
        }
        verified_handoff = {
            "receipt_sha256": reuse_result["source_handoff"]["receipt_sha256"],
            "receipt": {
                "implementation_source": reuse_result["deck_execution_source"],
                "bundle": {
                    "sha256": reuse_result["source_handoff"]["bundle_sha256"],
                    "advertised_ref": reuse_result["source_handoff"]["advertised_ref"],
                },
            },
        }
        validate_result(reuse_result)
        reuse_transaction = copy.deepcopy(transaction)
        reuse_transaction["transaction_result"] = reuse_result
        reuse_transaction["result_admission"] = result_admission_receipt(
            reuse_result, consumer_source=base["role"], renderer=renderer)
        reuse_transaction["windows_build_input"]["sha256"] = build_sha
        reuse_transaction["invalidation_results"]["corrective_history"][
            "v3_corrective_execution"]["result_sha256"] = sha256_bytes(
                canonical_json(reuse_result))
        reuse_receipt = pc0_renderer_reuse_proof(
            reuse_result, driver, reuse_transaction, fixture_override=fixture,
            host_override=host, verified_handoff=verified_handoff)
        if (set(reuse_receipt) != {
                "windows_builds", "artifact_downloads", "custody_operations",
                "artifact_transfers", "source_transfers", "deck_executions",
                "evidence_renders", "actual_retained_observation_reused",
                "consumer_mutation", "observation_source"}
                or any(reuse_receipt[key] for key in (
                    "windows_builds", "artifact_downloads", "custody_operations",
                    "artifact_transfers", "source_transfers", "deck_executions"))
                or reuse_receipt["evidence_renders"] != 1):
            fail("PC0 production renderer-only reuse proof differs")
    if any(effects.values()): fail("PC0 deterministic suite made external effects")
    return {"all_passed": True, "proof_row_count": 16,
            "invalidation_results": invalidation, "external_effects": effects,
            "renderer_only_production_path": True}


def _pc0_renderer_consumer_worker(payload: dict[str, Any]) -> dict[str, Any]:
    """Execute cache admission and rendering from the runnable consumer C checkout."""
    import host_proof
    from common import PC0_PLAN_ID
    result = payload["result"]
    transaction = payload["transaction"]
    host = payload["host"]
    fixture = payload["fixture"]
    receipt = payload["reuse_receipt"]
    validate_result(result)

    def forbidden(*args: Any, **kwargs: Any) -> None:
        fail("PC0 renderer-only correction attempted an external operation")

    consumer = host_proof.ProofTransactionDriver(
        payload["consumer_commit"], PC0_PLAN_ID,
        proof_root=pathlib.Path(payload["proof_root"]), github_factory=forbidden)
    host_proof._validate_result_store_joins(
        result, host, fixture, verified_handoff=payload["verified_handoff"])
    changed = copy.deepcopy(transaction)
    changed["evidence_consumer_source"] = consumer.source_role
    changed["evidence_renderer"] = consumer.renderer
    changed["result_admission"] = result_admission_receipt(
        result, consumer_source=consumer.source_role, renderer=consumer.renderer)
    changed["renderer_only_reuse"] = receipt
    changed["costs"] = {**changed["costs"], "evidence_renders": 1}
    changed["phase_dispositions"]["retrieve_and_retain_result"] = "reused"
    changed["invalidation_results"]["corrective_history"][
        "v3_corrective_execution"]["evidence_consumer_source"] = consumer.source_role
    packet_root = pathlib.Path(payload["packet_root"])
    render_packet(packet_root, changed)
    validate_packet(packet_root)
    packet = json.loads((packet_root / "TRANSACTION.json").read_bytes())
    if (packet["artifact_producer_source"] != result["artifact_producer_source"]
            or packet["deck_execution_source"] != result["deck_execution_source"]
            or packet["evidence_consumer_source"] != consumer.source_role
            or packet["renderer"] != consumer.renderer
            or packet["observation_disposition"] != "reused_original_observation"
            or packet["consumer_executed_on_deck"] is not False
            or packet["consumer_deck_state_freshly_inspected"] is not False):
        fail("PC0 rerender falsely relabelled the original observation")
    if any(consumer.effect_counts[key] for key in (
            "windows_builds", "artifact_downloads", "custody_operations",
            "artifact_transfers", "source_transfers", "deck_executions")):
        fail("PC0 renderer-only production path incurred external effects")
    return receipt


def pc0_renderer_reuse_proof(result: dict[str, Any], driver: Any,
                              transaction: dict[str, Any], *,
                              fixture_override: dict[str, Any] | None = None,
                              host_override: dict[str, Any] | None = None,
                              verified_handoff: dict[str, Any] | None = None) -> dict[str, Any]:
    """Run a real renderer-only C over the admitted original P/E observation."""
    import os
    import subprocess
    import sys
    from common import PC0_PLAN_ID, repo_root
    module = sys.modules[type(driver).__module__]
    validate_result(result)
    fixture = fixture_override or verify_fixture_store()
    host = host_override or driver._pc0_exact_host_cache()
    if verified_handoff is None:
        handoff_stage = (module.dx0_mac_transaction_parent()
                         / result["operation_nonce"] / "source-handoff")
        verified_handoff = module.pc0_v3_verify_source_handoff(
            handoff_stage, result["deck_execution_source"]["commit"])
    module._validate_result_store_joins(
        result, host, fixture, verified_handoff=verified_handoff)
    zero_effects = {key: 0 for key in (
        "windows_builds", "artifact_downloads", "custody_operations",
        "artifact_transfers", "source_transfers", "deck_executions")}
    receipt = {**zero_effects, "evidence_renders": 1,
        "actual_retained_observation_reused": True,
        "consumer_mutation": "synthetic_renderer_only_Git_revision",
        "observation_source": result["deck_execution_source"]["commit"]}
    with tempfile.TemporaryDirectory(prefix="pc0-retained-rerender-") as temporary:
        stage = pathlib.Path(temporary)
        consumer = _pc0_materialize_renderer_consumer(
            repo_root(), driver.source_commit, stage,
            module.pc0_v3_complete_source, module.PC0_V3_AUTHORITY_COMMIT)
        if (consumer["build"] != driver.build_input
                or consumer["deck"] != dx0_deck_execution_input(
                    driver.source_commit, "a" * 64, "b" * 64,
                    dx0_identity_sha256(dx0_closed_plan(PC0_PLAN_ID)))):
            fail("PC0_EVIDENCE_BLOCKED: renderer mutation changed build or Deck input")
        changed_input = dx0_deck_execution_input(
            consumer["source"]["commit"], host["manifest_sha256"],
            fixture["identity_sha256"], driver.plan_sha,
            root=consumer["checkout"])
        if dx0_identity_sha256(changed_input) != result["execution_input"]["identity_sha256"]:
            fail("PC0_EVIDENCE_BLOCKED: renderer reuse execution-input join differs")
        payload = {
            "consumer_commit": consumer["source"]["commit"], "result": result,
            "transaction": transaction, "host": host, "fixture": fixture,
            "verified_handoff": verified_handoff, "reuse_receipt": receipt,
            "proof_root": str(stage / "consumer-transactions"),
            "lock_root": str(stage / "locks"), "packet_root": str(stage / "packet"),
        }
        script = """\
import importlib.util, json, pathlib, sys
root = pathlib.Path.cwd()
sys.path.insert(0, str(root / 'tools/wf0-factory-census'))
spec = importlib.util.spec_from_file_location('host_proof', root / 'tools/host-proof.py')
module = importlib.util.module_from_spec(spec)
sys.modules['host_proof'] = module
spec.loader.exec_module(module)
from common import canonical_json
from negative_tests import _pc0_renderer_consumer_worker
value = json.loads(sys.stdin.buffer.read())
sys.stdout.buffer.write(canonical_json(_pc0_renderer_consumer_worker(value)))
"""
        environment = {**os.environ, "PYTHONDONTWRITEBYTECODE": "1"}
        completed = subprocess.run(
            [sys.executable, "-c", script], cwd=consumer["checkout"], env=environment,
            input=canonical_json(payload), stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            timeout=180, check=False)
        if completed.returncode != 0:
            diagnostic = completed.stderr.decode("utf-8", "replace")[-4000:]
            fail(f"PC0 renderer-only consumer failed: {diagnostic}")
        if json.loads(completed.stdout) != receipt:
            fail("PC0 renderer-only consumer receipt differs")
        # The exact runnable consumer C already validated this packet in its
        # own checkout, where its synthetic commit is resolvable. The parent
        # repository deliberately does not import that private test object.
        packet = json.loads((stage / "packet/TRANSACTION.json").read_bytes())
        if (packet["evidence_consumer_source"] != consumer["role"]
                or packet["renderer"] != consumer["renderer"]):
            fail("PC0 renderer-only packet was not produced by consumer C")
    return receipt

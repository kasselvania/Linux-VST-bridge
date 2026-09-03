#!/usr/bin/env python3
"""Deterministic DX0 identity, admission, invalidation, and recovery proofs."""

from __future__ import annotations

import ast
import copy
import json
import pathlib
import tempfile
from typing import Any

from artifacts import (
    accepted_fixture_identity, accepted_fixture_identity_sha256,
    create_source_handoff, verify_source_handoff,
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
    validate_result,
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

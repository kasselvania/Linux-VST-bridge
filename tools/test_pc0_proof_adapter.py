#!/usr/bin/env python3
"""Deterministic no-network tests for the closed PX3 PC0 diagnostic adapter."""

from __future__ import annotations

import copy
import pathlib
import subprocess
import sys
import tempfile
import unittest


TOOLS = pathlib.Path(__file__).resolve().parent
ROOT = TOOLS.parent
sys.path.insert(0, str(TOOLS))

from classified_proof_backend import (  # noqa: E402
    BudgetExhausted,
    ClassifiedProofBackend,
    DiagnosticPlanAdapter,
    PreflightFailed,
    TransactionState,
    UnsupportedAdapter,
)
from pc0_proof_adapter import (  # noqa: E402
    ACCEPTED_FIXTURE_IDENTITY,
    AdapterBoundaryError,
    AdapterPorts,
    AGAIN_BUNDLE_MANIFEST_SHA256,
    AGAIN_MODULE_SHA256,
    CommandReply,
    HOST_MANIFEST_SHA256,
    MAX_REMOTE_DOCUMENT_BYTES,
    PC0_DECK_EXECUTION_INPUT_SCHEMA,
    PC0_DIAGNOSTIC_KEYS,
    PC0_FAILURE_DIAGNOSTIC_SCHEMA,
    PC0_LEGACY_PLAN_SHA256,
    PC0_RESULT_SCHEMA,
    PLAN_CONTENT_SHA256,
    PLAN_DESCRIPTOR,
    PLAN_ID,
    PRODUCER_ARTIFACT_ID,
    PRODUCER_RUN_ATTEMPT,
    PRODUCER_RUN_ID,
    PRODUCER_SOURCE_ROLE,
    PRODUCT_CONTRACT_IDENTITY,
    PRODUCTION_ADAPTERS,
    REMOTE_PREFLIGHT_PROGRAM,
    RUNTIME_PROTON_IDENTITY,
    SELECTION_GIT_BLOB,
    SELECTION_PATH,
    SELECTION_SHA256,
    STOPPED_PC0_ARCHIVE_REF,
    STOPPED_PC0_SOURCE,
    STOPPED_PC0_TREE,
    STOPPED_SOURCE_ROLE,
    WINDOWS_BUILD_INPUT_IDENTITY,
    PC0DiagnosticRuntime,
    PortTimeout,
)
from proof_execution_policy import (  # noqa: E402
    ExecutionClass,
    LiveRequest,
    PolicyError,
    authorize_live_request,
    canonical_json,
    load_authority,
    parse_canonical_json,
    sha256_bytes,
)


SOURCE_A = "1" * 40
SOURCE_B = "2" * 40
SOURCE_C = "3" * 40
CAMPAIGN = "4" * 64
EXECUTION_INPUT = "5" * 64
RECEIPT_SHA = "6" * 64
BUNDLE_SHA = "7" * 64
PROTECTED_SHA = "8" * 64
RESULT_RUN_ID = "9" * 32
ZERO_CALL_COUNTS = {
    operation: 0 for operation in (
        "load_library", "init_dll", "get_plugin_factory", "get_factory_info",
        "query_factory_2", "query_factory_3", "count_classes",
        "get_class_info_unicode", "get_class_info_2", "get_class_info_1",
        "create_component", "get_controller_class_id", "initialize_component",
        "query_audio_processor", "get_bus_count", "get_bus_info",
        "get_bus_arrangement", "can_process_sample_size",
        "release_audio_processor", "terminate_component", "release_component",
        "release_factory_3", "release_factory_2", "release_factory_base",
        "exit_dll", "free_library",
    )
}


def authority_text(
    source: str = SOURCE_A, *, campaign: str = CAMPAIGN, budget: int = 1,
    plan_id: str = PLAN_ID, plan_sha256: str = PLAN_CONTENT_SHA256,
) -> str:
    fields = {
        "status": "active_diagnostic_campaign",
        "authority_phase": "proof_harness_maintenance",
        "change_class": "PROOF_HARNESS_MAINTENANCE",
        "product_implementation_authorized": "false",
        "live_execution_authorized": "true",
        "permitted_execution_class": "DIAGNOSTIC_NON_AUTHORITATIVE",
        "classified_backend_core_ready": "true",
        "classified_backend_ready": "true",
        "authorized_source_commit": source,
        "authorized_plan_id": plan_id,
        "authorized_product_contract_identity": PRODUCT_CONTRACT_IDENTITY,
        "authorized_product_contract_sha256": SELECTION_SHA256,
        "authorized_plan_content_sha256": plan_sha256,
        "diagnostic_campaign_identity": campaign,
        "diagnostic_batch_budget": str(budget),
    }
    body = "\n".join(f"{key}: {value}" for key, value in fields.items())
    return f"# Test authority\n\n## Authority\n\n```yaml\n{body}\n```\n"


def write_authority(
    root: pathlib.Path, source: str = SOURCE_A, *, campaign: str = CAMPAIGN,
    budget: int = 1, name: str = "authority.md", plan_id: str = PLAN_ID,
    plan_sha256: str = PLAN_CONTENT_SHA256,
):
    path = root / name
    path.write_text(authority_text(
        source, campaign=campaign, budget=budget,
        plan_id=plan_id, plan_sha256=plan_sha256,
    ), encoding="utf-8")
    authority = load_authority(path)
    delegation = authorize_live_request(authority, LiveRequest(
        ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE,
        source, plan_id, campaign,
    ))
    return authority, canonical_json(delegation)


class FakeCommandPort:
    def __init__(
        self, *, source_mismatch: bool = False, head_source: str = SOURCE_A,
    ) -> None:
        self.source_mismatch = source_mismatch
        self.head_source = head_source
        self.calls: list[tuple[str, ...]] = []

    def run(self, argv, *, cwd=None, timeout=30.0):
        del cwd, timeout
        command = tuple(argv)
        self.calls.append(command)
        target = command[-1]
        if command == ("git", "rev-parse", "HEAD"):
            value = self.head_source
        elif command == (
            "git", "status", "--porcelain=v1", "--untracked-files=all",
        ):
            return CommandReply(0, b"", b"")
        elif command[:3] == ("git", "rev-parse", "--verify"):
            if target == f"{STOPPED_PC0_ARCHIVE_REF}^{{commit}}":
                value = STOPPED_PC0_SOURCE
            else:
                value = target.removesuffix("^{commit}")
                if self.source_mismatch:
                    value = "f" * 40
        elif command[:2] == ("git", "rev-parse"):
            if target == f"{STOPPED_PC0_SOURCE}^{{tree}}":
                value = STOPPED_PC0_TREE
            elif target.endswith(f":{SELECTION_PATH}"):
                value = SELECTION_GIT_BLOB
            else:
                return CommandReply(2, b"", b"unsupported fake git command")
        else:
            return CommandReply(2, b"", b"non-git command refused")
        return CommandReply(0, (value + "\n").encode("ascii"), b"")


class FakeFileSystemPort:
    def __init__(self) -> None:
        self.selection = (ROOT / SELECTION_PATH).read_bytes()
        self.calls: list[pathlib.Path] = []

    def read_bytes(self, path: pathlib.Path, maximum: int) -> bytes:
        self.calls.append(path)
        if path.name != "SLICE_SELECTION.md" or len(self.selection) > maximum:
            raise AdapterBoundaryError("unexpected fake filesystem read")
        return self.selection


def result_document(operation_nonce: str, phase_nonce: str) -> bytes:
    value = {
        "schema": PC0_RESULT_SCHEMA,
        "operation_nonce": operation_nonce,
        "artifact_producer_source": dict(PRODUCER_SOURCE_ROLE),
        "deck_execution_source": dict(STOPPED_SOURCE_ROLE),
        "execution_input": {
            "identity_sha256": EXECUTION_INPUT,
            "schema": PC0_DECK_EXECUTION_INPUT_SCHEMA,
            "proof_plan_sha256": PC0_LEGACY_PLAN_SHA256,
            "runtime_proton_sha256": RUNTIME_PROTON_IDENTITY,
            "source_handoff_ref": f"refs/handoff/dx0-source/{STOPPED_PC0_SOURCE}",
            "detached_worktree_commit": STOPPED_PC0_SOURCE,
            "host_artifact_manifest_sha256": HOST_MANIFEST_SHA256,
            "accepted_fixture_identity_sha256": ACCEPTED_FIXTURE_IDENTITY,
        },
        "host_artifact": {
            "windows_build_input_sha256": WINDOWS_BUILD_INPUT_IDENTITY,
            "workflow_run_id": PRODUCER_RUN_ID,
            "run_attempt": PRODUCER_RUN_ATTEMPT,
            "artifact_id": PRODUCER_ARTIFACT_ID,
            "manifest_sha256": HOST_MANIFEST_SHA256,
            "build_receipt_sha256": "a" * 64,
            "mac_custody_receipt_sha256": "b" * 64,
        },
        "accepted_fixture": {
            "identity_sha256": ACCEPTED_FIXTURE_IDENTITY,
            "bundle_manifest_sha256": AGAIN_BUNDLE_MANIFEST_SHA256,
            "module_sha256": AGAIN_MODULE_SHA256,
            "mac_store_receipt_sha256": "c" * 64,
            "deck_store_receipt_sha256": "c" * 64,
        },
        "source_handoff": {
            "bundle_sha256": BUNDLE_SHA,
            "receipt_sha256": RECEIPT_SHA,
            "advertised_ref": f"refs/handoff/dx0-source/{STOPPED_PC0_SOURCE}",
            "worktree_commit": STOPPED_PC0_SOURCE,
            "worktree_clean": True,
        },
        "closed_plan": {
            "plan_id": "pc0-pre-setup-processing-contract-v1",
            "sha256": PC0_LEGACY_PLAN_SHA256,
            "expected_result": "pc0-pre-setup-contract-complete-v1",
            "live_exercise_ceiling": 1,
        },
        "original_observation": {
            "run_id": RESULT_RUN_ID,
            "phase_nonce": phase_nonce,
        },
        "positive_result": {"processing_contract": {
            "schema": "diagnostic-test-contract/v1", "audio_buses": [],
            "event_buses": [], "sample_sizes": [],
        }},
        "call_facts": {},
        "quiescence": {},
        "shutdown": {},
        "cleanup": {
            "owned_descendant_count": 0,
            "process_group_empty": True,
            "environment_retired": True,
            "stage_absent": True,
        },
        "protected_state": {
            "pre_sha256": PROTECTED_SHA,
            "post_sha256": PROTECTED_SHA,
            "equal": True,
            "comparison_completed": True,
        },
        "integrity": {},
    }
    return canonical_json(value)


def diagnostic_document(
    operation_nonce: str, phase_nonce: str, *,
    classification: str = "supervision_failed",
) -> bytes:
    shutdown = {
        operation: {
            "disposition": "not_attempted_prior_stage",
            "source": "supervisor_call_ledger_absence",
        }
        for operation in (
            "terminate_component", "release_component", "release_factory_3",
            "release_factory_2", "release_factory_base", "exit_dll",
            "free_library",
        )
    }
    value = {
        "schema": PC0_FAILURE_DIAGNOSTIC_SCHEMA,
        "operation_nonce": operation_nonce,
        "phase_nonce": phase_nonce,
        "execution_source": dict(STOPPED_SOURCE_ROLE),
        "execution_input_sha256": EXECUTION_INPUT,
        "proof_plan_sha256": PC0_LEGACY_PLAN_SHA256,
        "run_id": RESULT_RUN_ID,
        "raw_exit": None if classification == "supervision_failed" else 124,
        "classification": classification,
        "primary_blocker": "PC0_EVIDENCE_BLOCKED",
        "secondary_cleanup_blocker": None,
        "last_lifecycle": None,
        "last_in_flight_operation": None,
        "durable_record_count": 0,
        "durable_records": [],
        "call_counts": dict(ZERO_CALL_COUNTS),
        "audio_processor_observer_state": "audio_processor_absent",
        "audio_interface_quiescence": False,
        "inherited_shutdown": {
            "operations": shutdown,
            "clean_in_process_shutdown": False,
            "physical_containment_only": True,
        },
        "cleanup": {
            "owned_descendants_zero": True,
            "process_group_empty": True,
        },
        "environment_retirement_disposition": "retired",
        "stdout_sha256": "d" * 64,
        "stderr_sha256": "e" * 64,
        "stderr_bytes": 0,
        "protected_snapshot_sha256": PROTECTED_SHA,
        "runner_identity_sha256": RUNTIME_PROTON_IDENTITY,
    }
    assert set(value) == PC0_DIAGNOSTIC_KEYS
    return canonical_json(value)


class FakeSSHPort:
    def __init__(
        self, *, state: str = "absent", final_state: str = "result",
        overrides: dict[str, object] | None = None,
        diagnostic_classification: str = "supervision_failed",
        timeout_after_launch: bool = False,
        retained_nonces: tuple[str, str] | None = None,
    ) -> None:
        self.state = state
        self.final_state = final_state
        self.overrides = dict(overrides or {})
        self.diagnostic_classification = diagnostic_classification
        self.timeout_after_launch = timeout_after_launch
        self.retained_nonces = retained_nonces
        self.run_python_calls = 0
        self.execute_calls = 0
        self.writes: list[tuple[str, bytes]] = []
        self.last_operation_nonce = "0" * 32
        self.last_phase_nonce = "0" * 32

    def _set(self, value: dict, dotted: str, item: object) -> None:
        target = value
        parts = dotted.split(".")
        for part in parts[:-1]:
            target = target[part]
        target[parts[-1]] = item

    def _preflight(self, arguments) -> dict:
        source, adapter_source, operation_nonce, phase_nonce = arguments
        self.last_operation_nonce = operation_nonce
        self.last_phase_nonce = phase_nonce
        retained_operation, retained_phase = (
            self.retained_nonces or (operation_nonce, phase_nonce)
        )
        has_pair = self.state in {"result", "diagnostic"}
        plan = {
            "schema": "linux-vst-bridge-dx0-proof-plan/v1",
            "plan_id": "pc0-pre-setup-processing-contract-v1",
            "accepted_fixture_id": "wa0-again-accepted-v1",
            "host_mode": "host_only",
            "deterministic_validation_set": "pc0-pre-setup-deterministic-v1",
            "live_deck_batch": "pc0-positive-only-v1",
            "expected_result": "pc0-pre-setup-contract-complete-v1",
            "evidence_renderer": "pc0-five-file-renderer-v1",
        }
        deck_input = {
            "schema": PC0_DECK_EXECUTION_INPUT_SCHEMA,
            "host_artifact_manifest_sha256": HOST_MANIFEST_SHA256,
            "accepted_fixture_identity_sha256": ACCEPTED_FIXTURE_IDENTITY,
            "proof_plan_sha256": PC0_LEGACY_PLAN_SHA256,
            "runtime_proton_sha256": RUNTIME_PROTON_IDENTITY,
            "record_count": 0,
            "records": [],
        }
        intent = {
            "schema": "linux-vst-bridge-dx0-deck-intent/v1",
            "operation_nonce": operation_nonce,
            "phase_nonce": phase_nonce,
            "execution_source": dict(STOPPED_SOURCE_ROLE),
            "deck_execution_input": deck_input,
            "deck_execution_input_sha256": EXECUTION_INPUT,
            "proof_plan": plan,
            "proof_plan_sha256": PC0_LEGACY_PLAN_SHA256,
            "host_artifact_manifest_sha256": HOST_MANIFEST_SHA256,
            "accepted_fixture_identity_sha256": ACCEPTED_FIXTURE_IDENTITY,
            "source_handoff_receipt_sha256": RECEIPT_SHA,
        }
        value = {
            "schema": "linux-vst-bridge-pc0-classified-diagnostic-preflight/v1",
            "adapter_source_commit": adapter_source,
            "execution_source_commit": source,
            "execution_input_sha256": EXECUTION_INPUT,
            "legacy_proof_plan_sha256": PC0_LEGACY_PLAN_SHA256,
            "operation_nonce": operation_nonce,
            "phase_nonce": phase_nonce,
            "source_handoff_receipt_sha256": RECEIPT_SHA,
            "source_handoff_bundle_sha256": BUNDLE_SHA,
            "host": {
                "windows_build_input_identity": WINDOWS_BUILD_INPUT_IDENTITY,
                "producer_source_commit": PRODUCER_SOURCE_ROLE["commit"],
                "producer_run_id": PRODUCER_RUN_ID,
                "producer_run_attempt": PRODUCER_RUN_ATTEMPT,
                "artifact_id": PRODUCER_ARTIFACT_ID,
                "manifest_sha256": HOST_MANIFEST_SHA256,
            },
            "fixture": {
                "again_module_sha256": AGAIN_MODULE_SHA256,
                "again_bundle_manifest_sha256": AGAIN_BUNDLE_MANIFEST_SHA256,
                "accepted_fixture_identity_sha256": ACCEPTED_FIXTURE_IDENTITY,
            },
            "runtime_proton_identity_sha256": RUNTIME_PROTON_IDENTITY,
            "protected_snapshot_sha256": PROTECTED_SHA,
            "fixture_platform": {
                "hardware": "Steam Deck Galileo",
                "os": "SteamOS 3.8.16",
                "architecture": "x86_64",
                "read_only_mode": "enabled",
            },
            "process_counts": {
                key: 0 for key in (
                    "bitwig", "validator", "wine", "proton", "runtime",
                    "umu", "yabridge", "wf0",
                )
            },
            "write_effect_counts": {
                key: 0 for key in (
                    "environment_creations", "execution_intent_publications",
                    "execution_lock_creations", "result_publications",
                    "diagnostic_publications", "protected_state_mutations",
                    "deck_execution_reservations",
                    "deck_execution_count_increments", "proton_launches",
                )
            },
            "outcome_state": self.state,
            "outcome_operation_nonce": retained_operation if has_pair else None,
            "outcome_phase_nonce": retained_phase if has_pair else None,
            "intent": intent,
        }
        for path, item in self.overrides.items():
            self._set(value, path, item)
        return value

    def run_python(self, worktree, program, arguments, *, timeout):
        del program, timeout
        self.run_python_calls += 1
        if STOPPED_PC0_SOURCE not in worktree:
            raise AdapterBoundaryError("wrong stopped worktree")
        return canonical_json(self._preflight(arguments))

    def execute_pc0(self, worktree, intent, *, timeout):
        del timeout
        self.execute_calls += 1
        if (STOPPED_PC0_SOURCE not in worktree
                or not intent.endswith("/DX0_DECK_EXECUTION_INTENT.json")):
            raise AdapterBoundaryError("wrong lower-level execution seam")
        self.state = "unknown" if self.timeout_after_launch else self.final_state
        if self.timeout_after_launch:
            raise PortTimeout("lost acknowledgement")
        if self.final_state == "result":
            return CommandReply(0, b'{"disposition":"completed"}\n', b"")
        return CommandReply(1, b"", b"bounded diagnostic failure")

    def read_file(self, path, maximum, *, timeout):
        del timeout
        operation_nonce, phase_nonce = (
            self.retained_nonces
            or (self.last_operation_nonce, self.last_phase_nonce)
        )
        if "DX0_TRANSACTION_RESULT" in path:
            raw = result_document(operation_nonce, phase_nonce)
            filename = "DX0_TRANSACTION_RESULT.json"
        else:
            raw = diagnostic_document(
                operation_nonce, phase_nonce,
                classification=self.diagnostic_classification,
            )
            filename = "PC0_FAILURE_DIAGNOSTIC.json"
        value = (
            f"{sha256_bytes(raw)}  {filename}\n".encode("ascii")
            if path.endswith(".sha256") else raw
        )
        if len(value) > maximum:
            raise AdapterBoundaryError("fake remote read exceeded bound")
        return value

    def write_file(self, path, data, *, timeout):
        del timeout
        self.writes.append((path, data))


class TestRuntime(PC0DiagnosticRuntime):
    def __init__(self, *args, store_error: bool = False, **kwargs) -> None:
        super().__init__(*args, **kwargs)
        self.store_error = store_error

    def _validate_mac_stores(self) -> None:
        if self.store_error:
            raise AdapterBoundaryError("injected Mac store mismatch")


def injected_adapter(
    base: pathlib.Path, ssh: FakeSSHPort, *, source_mismatch: bool = False,
    store_error: bool = False, head_source: str = SOURCE_A,
) -> tuple[DiagnosticPlanAdapter, FakeCommandPort, FakeFileSystemPort, TestRuntime]:
    command = FakeCommandPort(
        source_mismatch=source_mismatch, head_source=head_source,
    )
    filesystem = FakeFileSystemPort()
    runtime = TestRuntime(
        AdapterPorts(command, filesystem, ssh), repository=ROOT,
        proof_root=base / "proof", store_error=store_error,
    )
    adapter = DiagnosticPlanAdapter(
        descriptor=PLAN_DESCRIPTOR,
        preflight=runtime.preflight,
        reconcile=runtime.reconcile,
        invoke=runtime.invoke_diagnostic,
        admit=runtime.admit_diagnostic,
    )
    return adapter, command, filesystem, runtime


class PC0AdapterTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.base = pathlib.Path(self.temporary.name)

    def execute(
        self, ssh: FakeSSHPort, *, source: str = SOURCE_A,
        campaign: str = CAMPAIGN, budget: int = 1,
        state_name: str = "state", source_mismatch: bool = False,
        store_error: bool = False,
    ):
        authority, delegation = write_authority(
            self.base, source, campaign=campaign, budget=budget,
            name=f"authority-{source}-{campaign[:4]}.md",
        )
        adapter, command, filesystem, runtime = injected_adapter(
            self.base, ssh, source_mismatch=source_mismatch,
            store_error=store_error, head_source=source,
        )
        backend = ClassifiedProofBackend(
            self.base / state_name, {PLAN_ID: adapter},
        )
        return backend.execute(authority, delegation), backend, adapter, runtime

    def test_production_registry_is_exactly_one_diagnostic_plan(self):
        self.assertEqual(set(PRODUCTION_ADAPTERS), {PLAN_ID})
        adapter = PRODUCTION_ADAPTERS[PLAN_ID]
        self.assertIsInstance(adapter, DiagnosticPlanAdapter)
        self.assertFalse(hasattr(adapter, "render_product_evidence"))
        record = adapter.descriptor.record()
        self.assertEqual(record["execution_class"], "DIAGNOSTIC_NON_AUTHORITATIVE")
        self.assertEqual(record["product_contract_identity"], "pc0-selection-v2")
        self.assertEqual(record["product_contract_sha256"], SELECTION_SHA256)
        self.assertEqual(record["plan_content_sha256"], PLAN_CONTENT_SHA256)

    def test_current_no_active_authority_blocks_before_any_adapter_port(self):
        authority = load_authority(ROOT / "CURRENT_SLICE.md")
        with self.assertRaisesRegex(PolicyError, "LIVE_EXECUTION_FORBIDDEN"):
            authorize_live_request(authority, LiveRequest(
                ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE,
                SOURCE_A, PLAN_ID, CAMPAIGN,
            ))
        result = subprocess.run(
            [sys.executable, str(TOOLS / "proof-run.py"), "diagnose",
             "--source", SOURCE_A, "--plan", PLAN_ID,
             "--campaign", CAMPAIGN],
            stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, check=False,
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("LIVE_EXECUTION_FORBIDDEN", result.stderr)

    def test_unsupported_or_mismatched_inputs_fail_before_reservation(self):
        cases = (
            ("source", {"source_mismatch": True}),
            ("mac-store", {"store_error": True}),
            ("artifact", {"overrides": {"host.artifact_id": 1}}),
            ("fixture", {"overrides": {"fixture.again_module_sha256": "0" * 64}}),
            ("runtime", {"overrides": {"runtime_proton_identity_sha256": "0" * 64}}),
            ("plan", {"overrides": {"legacy_proof_plan_sha256": "0" * 64}}),
        )
        for index, (label, values) in enumerate(cases):
            with self.subTest(label=label):
                ssh = FakeSSHPort(overrides=values.get("overrides"))
                authority, delegation = write_authority(
                    self.base, name=f"mismatch-{index}.md",
                )
                adapter, _command, _filesystem, _runtime = injected_adapter(
                    self.base, ssh,
                    source_mismatch=bool(values.get("source_mismatch")),
                    store_error=bool(values.get("store_error")),
                )
                state = self.base / f"state-mismatch-{index}"
                backend = ClassifiedProofBackend(state, {PLAN_ID: adapter})
                with self.assertRaises(PreflightFailed):
                    backend.execute(authority, delegation)
                self.assertFalse(state.exists())
                self.assertEqual(ssh.execute_calls, 0)

        other_authority, other_delegation = write_authority(
            self.base, plan_id="unsupported-diagnostic-plan",
            name="unsupported.md",
        )
        unsupported_state = self.base / "state-unsupported"
        with self.assertRaises(UnsupportedAdapter):
            ClassifiedProofBackend(
                unsupported_state, dict(PRODUCTION_ADAPTERS),
            ).execute(other_authority, other_delegation)
        self.assertFalse(unsupported_state.exists())

    def test_preflight_is_read_only_and_failure_consumes_no_campaign_batch(self):
        compile(REMOTE_PREFLIGHT_PROGRAM, "<pc0-read-only-preflight>", "exec")
        for forbidden in (
            "create_dx0_environment", "write_atomic", "retire_environment",
            "supervise(", ".mkdir(", "os.replace",
        ):
            self.assertNotIn(forbidden, REMOTE_PREFLIGHT_PROGRAM)
        ssh = FakeSSHPort(overrides={"process_counts.wine": 1})
        authority, delegation = write_authority(self.base, name="preflight.md")
        adapter, _command, _filesystem, _runtime = injected_adapter(self.base, ssh)
        state = self.base / "state-preflight"
        with self.assertRaises(PreflightFailed):
            ClassifiedProofBackend(state, {PLAN_ID: adapter}).execute(
                authority, delegation,
            )
        self.assertEqual(ssh.writes, [])
        self.assertEqual(ssh.execute_calls, 0)
        self.assertEqual(list(state.rglob("budget.json")), [])

    def test_diagnostic_source_revisions_share_one_campaign_budget(self):
        ssh = FakeSSHPort(
            state="result", retained_nonces=("a" * 32, "b" * 32),
        )
        adapter, command, _filesystem, _runtime = injected_adapter(self.base, ssh)
        backend = ClassifiedProofBackend(self.base / "state-campaign", {PLAN_ID: adapter})
        authorities = [
            write_authority(
                self.base, source, budget=2, name=f"revision-{index}.md",
            )
            for index, source in enumerate((SOURCE_A, SOURCE_B, SOURCE_C), 1)
        ]
        command.head_source = SOURCE_A
        first = backend.execute(*authorities[0])
        command.head_source = SOURCE_B
        second = backend.execute(*authorities[1])
        self.assertEqual((first.budget_consumed, second.budget_consumed), (1, 2))
        with self.assertRaises(BudgetExhausted):
            command.head_source = SOURCE_C
            backend.execute(*authorities[2])
        self.assertEqual(ssh.execute_calls, 0)
        budget_path = self.base / "state-campaign" / "diagnostic" / CAMPAIGN / "budget.json"
        budget = parse_canonical_json(budget_path.read_bytes(), maximum=512 * 1024)
        self.assertEqual(budget["consumed_count"], 2)

    def test_retained_result_and_diagnostic_reconcile_without_relaunch(self):
        for index, (state, classification) in enumerate((
            ("result", "supervision_failed"),
            ("diagnostic", "supervision_failed"),
        ), 1):
            with self.subTest(state=state):
                ssh = FakeSSHPort(
                    state=state, diagnostic_classification=classification,
                    retained_nonces=("a" * 32, "b" * 32),
                )
                receipt, _backend, adapter, _runtime = self.execute(
                    ssh, campaign=str(index + 4) * 64,
                    state_name=f"state-retained-{state}",
                )
                self.assertEqual(receipt.state, TransactionState.CLOSED.value)
                self.assertEqual(ssh.execute_calls, 0)
                self.assertFalse(receipt.renderer_completed)
                self.assertFalse(hasattr(adapter, "render_product_evidence"))
                if state == "result":
                    self.assertIsNotNone(receipt.result_sha256)
                else:
                    self.assertIsNotNone(receipt.failure_sha256)

    def test_unknown_outcome_is_consumed_and_never_relaunched(self):
        ssh = FakeSSHPort(timeout_after_launch=True)
        authority, delegation = write_authority(self.base, name="unknown.md")
        adapter, _command, _filesystem, _runtime = injected_adapter(self.base, ssh)
        backend = ClassifiedProofBackend(self.base / "state-unknown", {PLAN_ID: adapter})
        first = backend.execute(authority, delegation)
        second = backend.execute(authority, delegation)
        self.assertEqual(first.state, TransactionState.CLOSED.value)
        self.assertEqual(second.budget_consumed, 1)
        self.assertEqual(ssh.execute_calls, 1)
        transaction = (self.base / "state-unknown" / "diagnostic" / CAMPAIGN
                       / "transactions" / first.reservation_identity)
        failure = parse_canonical_json(
            (transaction / "classified-failure.json").read_bytes(),
        )
        self.assertEqual(failure["effects"]["deck_workloads"], "unknown")

    def test_every_outcome_is_acceptance_ineligible_and_renderer_absent(self):
        cases = (
            ("result", "supervision_failed", True),
            ("diagnostic", "call_timeout", False),
            ("diagnostic", "supervision_failed", False),
        )
        for index, (state, classification, success) in enumerate(cases, 6):
            with self.subTest(state=state, classification=classification):
                ssh = FakeSSHPort(
                    state=state, diagnostic_classification=classification,
                    retained_nonces=("a" * 32, "b" * 32),
                )
                receipt, _backend, adapter, _runtime = self.execute(
                    ssh, campaign=str(index) * 64,
                    state_name=f"state-outcome-{index}",
                )
                self.assertFalse(hasattr(adapter, "render_product_evidence"))
                transaction_path = next(
                    (self.base / f"state-outcome-{index}").rglob("transaction.json")
                )
                transaction = parse_canonical_json(
                    transaction_path.read_bytes(), maximum=512 * 1024,
                )
                self.assertFalse(transaction["delegation"]["acceptance_eligible"])
                self.assertFalse(transaction["observation"]["payload"]["acceptance_eligible"])
                self.assertFalse(receipt.renderer_completed)
                self.assertEqual(receipt.result_sha256 is not None, success)

    def test_diagnostic_pair_rejects_noncanonical_sidecar_mismatch_and_oversize(self):
        ssh = FakeSSHPort()
        _adapter, _command, _filesystem, runtime = injected_adapter(self.base, ssh)
        operation_nonce, phase_nonce = "a" * 32, "b" * 32
        raw = diagnostic_document(operation_nonce, phase_nonce)
        sidecar = (
            f"{sha256_bytes(raw)}  PC0_FAILURE_DIAGNOSTIC.json\n".encode("ascii")
        )
        summary, _digest, _kind, _cleanup = runtime._validate_diagnostic_pair(
            raw, sidecar, EXECUTION_INPUT, operation_nonce, phase_nonce,
            PROTECTED_SHA,
        )
        self.assertEqual(summary["classification"], "supervision_failed")

        malformed = copy.deepcopy(parse_canonical_json(raw, maximum=MAX_REMOTE_DOCUMENT_BYTES))
        malformed["durable_record_count"] = 1
        malformed_raw = canonical_json(malformed)
        failures = (
            (raw.rstrip(b"\n"), sidecar),
            (raw, b"0" * 64 + b"  PC0_FAILURE_DIAGNOSTIC.json\n"),
            (malformed_raw, f"{sha256_bytes(malformed_raw)}  PC0_FAILURE_DIAGNOSTIC.json\n".encode()),
            (b"x" * (MAX_REMOTE_DOCUMENT_BYTES + 1), sidecar),
        )
        for index, (bad_raw, bad_sidecar) in enumerate(failures):
            with self.subTest(case=index):
                with self.assertRaises(AdapterBoundaryError):
                    runtime._validate_diagnostic_pair(
                        bad_raw, bad_sidecar, EXECUTION_INPUT,
                        operation_nonce, phase_nonce, PROTECTED_SHA,
                    )

    def test_only_lower_level_execute_seam_is_present_and_no_evidence_packet_is_created(self):
        source = (TOOLS / "pc0_proof_adapter.py").read_text(encoding="utf-8")
        self.assertIn('"tools/wf0-factory-census/run.py"', source)
        self.assertNotIn('"tools/host-proof.py"', source)
        self.assertNotIn('"tools/wf0-factory-census/evidence.py"', source)
        self.assertNotIn("render_product_evidence=", source)

        ssh = FakeSSHPort(state="result", retained_nonces=("a" * 32, "b" * 32))
        receipt, _backend, _adapter, _runtime = self.execute(
            ssh, campaign="f" * 64, state_name="state-no-evidence",
        )
        self.assertIsNotNone(receipt.result_sha256)
        names = {path.name for path in (self.base / "state-no-evidence").rglob("*")}
        self.assertFalse(any(name.startswith("PC0_EVIDENCE") for name in names))
        self.assertFalse(any("COST_AND_INVALIDATION" in name for name in names))


if __name__ == "__main__":
    unittest.main(verbosity=2)

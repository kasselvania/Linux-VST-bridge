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

    def run(self, argv, *, cwd=None, timeout=30.0, input_bytes=None):
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
            elif target.endswith(":tools/pc0_diagnostic_worker.py"):
                value = adapter_module._git_blob_sha1(REMOTE_PREFLIGHT_PROGRAM.encode())
            elif target.endswith(f":{SELECTION_PATH}"):
                value = SELECTION_GIT_BLOB
            else:
                return CommandReply(2, b"", b"unsupported fake git command")
        else:
            return CommandReply(2, b"", b"non-git command refused")
        return CommandReply(0, (value + "\n").encode("ascii"), b"")


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


# Real helper contracts are read from the pinned local Git object, never from
# an artifact download. CI fetches history so this exact object is available.
import contextlib
import importlib
import io
import json
import shlex
from unittest.mock import patch, create_autospec
import pc0_proof_adapter as adapter_module
from pc0_proof_adapter import OSFileSystemPort, StrictSSHPort


class LocalWorkerPort:
    """Execute the actual stdin loader AND worker; only effectful primitives mocked."""
    def __init__(self, test):
        self.test = test
        self.calls = []
        self.actions = []
        self.lose_ack = False

    def run(self, argv, *, cwd=None, timeout=30.0, input_bytes=None):
        self.calls.append((tuple(argv), input_bytes))
        tokens = shlex.split(argv[-1])
        index = tokens.index("-c")
        loader, python_args = tokens[index + 1], tokens[index + 2:]
        self.actions.append(python_args[1])
        output = io.BytesIO()
        with contextlib.ExitStack() as stack:
            stack.enter_context(patch.object(sys, "argv", ["-c", *python_args]))
            stack.enter_context(patch.object(sys, "stdin", io.TextIOWrapper(io.BytesIO(input_bytes))))
            stack.enter_context(patch.object(sys, "stdout", io.TextIOWrapper(output, write_through=True)))
            stack.enter_context(patch.object(pathlib.Path, "home", return_value=self.test.remote_home))
            exec(loader, {"__name__": "__main__"})
            raw = output.getvalue()
        if self.lose_ack and python_args[1] == "execute":
            self.lose_ack = False
            raise PortTimeout("lost acknowledgement after worker publication")
        return CommandReply(0, raw, b"")


class PC0AdapterTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.modules_temp = tempfile.TemporaryDirectory()
        cls.module_root = pathlib.Path(cls.modules_temp.name)
        for name in ("common", "artifacts", "environment", "normalize", "supervise", "run"):
            data = subprocess.check_output([
                "git", "show", f"{STOPPED_PC0_SOURCE}:tools/wf0-factory-census/{name}.py"], cwd=ROOT)
            (cls.module_root / f"{name}.py").write_bytes(data)
        cls.modules_patch = patch.dict(sys.modules)
        cls.modules_patch.start()
        sys.path.insert(0, str(cls.module_root))
        for name in ("common", "artifacts", "environment", "normalize", "supervise", "run"):
            sys.modules.pop(name, None)
        cls.common = importlib.import_module("common")
        cls.artifacts = importlib.import_module("artifacts")
        cls.run_module = importlib.import_module("run")

    @classmethod
    def tearDownClass(cls):
        sys.path.remove(str(cls.module_root))
        cls.modules_patch.stop()
        cls.modules_temp.cleanup()

    def mock(self, owner, name, **kwargs):
        original = getattr(owner, name)
        replacement = create_autospec(original, **kwargs)
        self.stack.enter_context(patch.object(owner, name, replacement))
        return replacement

    def pair(self, path, value):
        path.parent.mkdir(parents=True, exist_ok=True)
        raw = canonical_json(value)
        path.write_bytes(raw)
        path.with_suffix(".sha256").write_bytes(f"{sha256_bytes(raw)}  {path.name}\n".encode())
        return sha256_bytes(raw)

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.base = pathlib.Path(self.temporary.name).resolve()
        self.stack = contextlib.ExitStack()
        self.addCleanup(self.stack.close)
        self.remote_home = self.base / "remote"
        self.remote_proof = self.remote_home / ".local/share/linux-vst-bridge/proof"
        self.mac_proof = self.base / "proof"
        self.busy = False
        self.stale = False
        self.protected_bad = False
        self.fail_supervise = False
        self.failure = False
        self.contained = True
        self.failed_retirement = False
        self.launches = 0
        self.protected = {"fixture": "local-effect-free"}
        self.module_bytes = b"local inert module identity fixture"
        module_sha = sha256_bytes(self.module_bytes)
        manifest = {"windows_build_input_sha256": WINDOWS_BUILD_INPUT_IDENTITY}
        host_sha = sha256_bytes(canonical_json(manifest))
        fixture_receipt = {"bundle_manifest": {"sha256": AGAIN_BUNDLE_MANIFEST_SHA256,
            "records": [{"path": "Contents/x86_64-win/again.vst3", "sha256": module_sha}]}}
        fixture_sha = sha256_bytes(canonical_json(fixture_receipt))
        for name, value in (("HOST_MANIFEST_SHA256", host_sha),
                            ("ACCEPTED_FIXTURE_IDENTITY", fixture_sha), ("AGAIN_MODULE_SHA256", module_sha)):
            self.stack.enter_context(patch.object(adapter_module, name, value))
        self.stack.enter_context(patch.object(self.common, "DX0_AGAIN_MODULE_SHA256", module_sha))
        host = self.mac_proof / "host-artifacts/by-manifest" / host_sha
        build = {"producer_source": dict(PRODUCER_SOURCE_ROLE),
                 "windows_build_input": {"sha256": WINDOWS_BUILD_INPUT_IDENTITY},
                 "workflow": {"run_id": PRODUCER_RUN_ID, "run_attempt": PRODUCER_RUN_ATTEMPT},
                 "artifact": {"manifest_sha256": host_sha}}
        custody = {"producer_source": dict(PRODUCER_SOURCE_ROLE),
                   "windows_build_input_sha256": WINDOWS_BUILD_INPUT_IDENTITY,
                   "workflow": build["workflow"], "artifact": {"id": PRODUCER_ARTIFACT_ID},
                   "inner_envelope": {"host_artifact_manifest_sha256": host_sha}}
        self.pair(host / "DX0_HOST_ARTIFACT_MANIFEST.json", manifest)
        self.pair(host / "DX0_WINDOWS_HOST_BUILD_RECEIPT.json", build)
        self.pair(host / "DX0_MAC_HOST_CUSTODY_RECEIPT.json", custody)
        fixture = self.mac_proof / "fixtures/by-manifest" / AGAIN_BUNDLE_MANIFEST_SHA256
        self.pair(fixture / "DX0_ACCEPTED_FIXTURE_RECEIPT.json", fixture_receipt)
        module = fixture / "again.vst3/Contents/x86_64-win/again.vst3"
        module.parent.mkdir(parents=True)
        module.write_bytes(self.module_bytes)
        self.host_path = host
        self.fixture_path = fixture
        self.host = {"root": str(host), "manifest_sha256": host_sha, "build_receipt": build,
                     "custody": custody, "build_receipt_sha256": "a" * 64, "custody_sha256": "b" * 64}
        self.fixture = {"root": str(fixture), "identity_sha256": fixture_sha,
                        "identity": fixture_receipt, "receipt_sha256": fixture_sha}
        self.source = {"commit": STOPPED_PC0_SOURCE}
        self.mock(self.run_module, "pc0_v3_require_frozen_source", return_value=self.source)
        self.mock(self.common, "dx0_source_role", return_value=dict(STOPPED_SOURCE_ROLE))
        self.mock(self.run_module, "pc0_v3_verify_source_handoff", return_value={"receipt_sha256": RECEIPT_SHA})
        self.mock(self.common, "dx0_deck_source_parent", return_value=self.base / "handoffs")
        self.mock(self.common, "dx0_deck_host_artifact_parent", return_value=host.parent)
        self.mock(self.common, "dx0_deck_fixture_parent", return_value=fixture.parent)
        # Actual artifacts.read_canonical_json is intentionally NOT mocked.
        self.host_reader = self.mock(self.artifacts, "verify_host_store", return_value=self.host)
        self.fixture_reader = self.mock(self.artifacts, "verify_fixture_store", return_value=self.fixture)
        self.mock(self.common, "deck_fixture_identity", return_value={})
        self.mock(self.common, "verify_runner_identity", return_value={"launch_critical_manifest_sha256": RUNTIME_PROTON_IDENTITY})
        self.mock(self.common, "protected_snapshot", side_effect=self.snapshot)
        # Actual inherited guard executes on the argv generated by StrictSSHPort.
        self.mock(self.common, "process_census", side_effect=lambda: [
            {"comm": "python3", "cmdline": shlex.join(sys.argv)},
            {"comm": "proton" if self.busy else "idle", "cmdline": ""}])
        env_parent = self.base / "environments"
        env_parent.mkdir()
        self.mock(self.common, "environment_parent", return_value=env_parent)
        self.environment_parent = env_parent
        self.mock(self.common, "dx0_deck_execution_input", return_value={"records": [], "record_count": 0})
        self.create = self.mock(self.run_module, "create_dx0_environment", side_effect=lambda *args, **kwargs: object())
        self.supervise = self.mock(self.run_module, "supervise", side_effect=self.scan)
        self.retire = self.mock(self.run_module, "retire_environment", side_effect=self.retirement)
        self.mock(self.run_module, "_build_for_normalizer", return_value={})
        self.mock(self.run_module, "normalize_wa0_positive", return_value=({"processing_contract": {}}, {}, {}))
        # Pinned failure validation and sanitized timeline remain real.
        self.command = FakeCommandPort()
        self.local_worker = LocalWorkerPort(self)
        ssh = StrictSSHPort(self.local_worker)
        ssh._destination = "local-effect-free-port"
        self.runtime = PC0DiagnosticRuntime(AdapterPorts(self.command, OSFileSystemPort(), ssh),
                                            repository=ROOT, proof_root=self.mac_proof)
        self.adapter = DiagnosticPlanAdapter(descriptor=PLAN_DESCRIPTOR, preflight=self.runtime.preflight,
            reconcile=self.runtime.reconcile, invoke=self.runtime.invoke_diagnostic, admit=self.runtime.admit_diagnostic)
        self.backend = ClassifiedProofBackend(self.base / "state", {PLAN_ID: self.adapter})

    def snapshot(self):
        if self.protected_bad:
            raise RuntimeError("protected-state check failed")
        return self.protected

    def retirement(self, environment):
        if self.failed_retirement:
            raise RuntimeError("retirement failed")
        return {"environment_retired": True, "stage_absent": True}

    def scan(self, environment, *, mode):
        self.assertEqual(mode, self.common.PC0_MODE)
        self.launches += 1
        if self.fail_supervise:
            raise RuntimeError("unresolved local supervision stand-in")
        diagnostic = parse_canonical_json(diagnostic_document("a" * 32, "b" * 32))
        return {**diagnostic, "run_id": f"{self.launches:032x}",
                "classification": "supervision_failed" if self.failure else "scanner_completed",
                "blocker": "PC0_EVIDENCE_BLOCKED" if self.failure else None,
                "cleanup": {"owned_descendants_zero": self.contained, "process_group_empty": self.contained},
                "protected_snapshot": self.protected,
                "runner_identity": {"launch_critical_manifest_sha256": RUNTIME_PROTON_IDENTITY},
                "records": []}

    def execute(self, source=SOURCE_A, budget=2):
        self.command.head_source = source
        authority, delegation = write_authority(self.base, source, budget=budget)
        return self.backend.execute(authority, delegation)

    def transaction(self, receipt):
        path = self.base / "state/diagnostic" / CAMPAIGN / "transactions" / receipt.reservation_identity / "transaction.json"
        return parse_canonical_json(path.read_bytes(), maximum=512 * 1024)

    def test_real_store_sidecars_and_hashes(self):
        self.runtime._validate_mac_stores()
        for directory, name in ((self.host_path, "DX0_HOST_ARTIFACT_MANIFEST"),
                                (self.host_path, "DX0_WINDOWS_HOST_BUILD_RECEIPT"),
                                (self.host_path, "DX0_MAC_HOST_CUSTODY_RECEIPT"),
                                (self.fixture_path, "DX0_ACCEPTED_FIXTURE_RECEIPT")):
            with self.subTest(name=name):
                sidecar = directory / (name + ".sha256")
                raw = sidecar.read_bytes()
                wrong = directory / (name + ".json.sha256")
                sidecar.rename(wrong)
                with self.assertRaises(AdapterBoundaryError):
                    self.runtime._validate_mac_stores()
                wrong.rename(sidecar)
                sidecar.write_bytes(b"0" * len(raw))
                with self.assertRaises(AdapterBoundaryError):
                    self.runtime._validate_mac_stores()
                sidecar.write_bytes(raw)

    def test_real_helper_stdin_and_actual_reader_guard(self):
        receipt = self.execute()
        self.assertEqual(receipt.state, "CLOSED")
        self.assertEqual(self.launches, 1)
        self.assertTrue(self.host_reader.called)
        for argv, data in self.local_worker.calls:
            self.assertNotIn("proton", " ".join(argv).lower())
            self.assertEqual(data, REMOTE_PREFLIGHT_PROGRAM.encode())
        # The owning artifacts reader rejects a noncanonical retained receipt.
        path = self.host_path / "DX0_WINDOWS_HOST_BUILD_RECEIPT.json"
        path.write_bytes(path.read_bytes().rstrip(b"\n"))
        self.command.head_source = SOURCE_B
        with self.assertRaises(PreflightFailed):
            self.execute(SOURCE_B)
        self.assertEqual(self.launches, 1)

    def test_guard_stale_stage_and_protected_checks_block_new_launch(self):
        for failure in ("busy", "stale", "protected"):
            with self.subTest(failure=failure):
                self.busy = failure == "busy"
                self.protected_bad = failure == "protected"
                stage = self.environment_parent / ".wf0-factory-census.stage-local"
                if failure == "stale":
                    stage.mkdir()
                with self.assertRaises(PreflightFailed):
                    self.execute()
                self.assertEqual(self.launches, 0)
                self.assertFalse((self.base / "state").exists())
                if stage.exists():
                    stage.rmdir()

    def test_two_reservations_same_artifact_resume_and_third_refused(self):
        old = self.remote_proof / "results/by-execution-input/historical"
        old.mkdir(parents=True)
        historical = old / "DX0_TRANSACTION_RESULT.json"
        historical.write_bytes(b"original historical acceptance identity\n")
        before = historical.read_bytes()
        first = self.execute()
        repeat = self.execute()
        second = self.execute(SOURCE_B)
        self.assertEqual((first.budget_consumed, repeat.budget_consumed, second.budget_consumed), (1, 1, 2))
        self.assertEqual(self.launches, 2)
        self.assertNotEqual(first.reservation_identity, second.reservation_identity)
        with self.assertRaises(BudgetExhausted):
            self.execute(SOURCE_C)
        self.assertEqual(self.launches, 2)
        publications = list(self.remote_proof.glob("diagnostics/*/*/*/observation.json"))
        self.assertEqual(len(publications), 2)
        observations = [json.loads(path.read_bytes()) for path in publications]
        self.assertEqual(len({v["summary"]["run_id"] for v in observations}), 2)
        self.assertEqual(len({v["binding"]["host_manifest_sha256"] for v in observations}), 1)
        self.assertTrue(all(v["binding"]["acceptance_eligible"] is False for v in observations))
        self.assertEqual(historical.read_bytes(), before)
        self.assertEqual(list(old.iterdir()), [historical])

    def test_unknown_consumed_then_late_exact_result_one_launch(self):
        self.local_worker.lose_ack = True
        # Hold back the exact result pair after actual worker execution, simulating
        # an acknowledgement loss while bounded publication is still in flight.
        original = self.local_worker.run
        saved = []
        def delayed(*args, **kwargs):
            try:
                return original(*args, **kwargs)
            except PortTimeout:
                for path in self.remote_proof.glob("diagnostics/*/*/*/observation*"):
                    saved.append((path, path.read_bytes()))
                    path.unlink()
                raise
        self.local_worker.run = delayed
        first = self.execute()
        second = self.execute()
        self.assertEqual(first.state, "OUTCOME_UNKNOWN")
        self.assertEqual(second.state, "OUTCOME_UNKNOWN")
        self.assertEqual(second.budget_consumed, 1)
        self.assertIsNone(self.transaction(first)["observation"])
        self.assertEqual(self.launches, 1)
        for path, raw in saved:
            path.write_bytes(raw)
        self.busy = self.protected_bad = True
        (self.host_path / "DX0_HOST_ARTIFACT_MANIFEST.sha256").write_bytes(b"bad sidecar")
        recovered = self.execute()
        self.assertEqual(recovered.state, "CLOSED")
        self.assertEqual(self.launches, 1)
        observation = self.transaction(recovered)["observation"]
        self.assertEqual(observation["effects"], {"deck_workloads": 0, "diagnostic_publications": 0})
        self.assertFalse(observation["payload"]["acceptance_eligible"])

    def test_unknown_supervision_never_relaunches(self):
        self.fail_supervise = True
        first, second = self.execute(), self.execute()
        self.assertEqual((first.state, second.state), ("OUTCOME_UNKNOWN", "OUTCOME_UNKNOWN"))
        self.assertEqual(self.launches, 1)
        self.assertEqual(self.retire.call_count, 0)

    def test_existing_diagnostic_retrieval_ignores_failed_launch_safety(self):
        self.failure = True
        first = self.execute()
        self.assertEqual(first.state, "CLOSED")
        self.assertIsNotNone(first.failure_sha256)
        self.busy = self.protected_bad = True
        repeat = self.execute()
        self.assertEqual(repeat.state, "CLOSED")
        self.assertEqual(self.launches, 1)
        self.assertEqual(list(self.remote_proof.glob("results/**/*")), [])

    def test_other_reservation_cannot_relabel_diagnostic(self):
        self.execute()
        publication = next(self.remote_proof.glob("diagnostics/*/*/*/observation.json"))
        value = json.loads(publication.read_bytes())
        value["binding"]["campaign_identity"] = "0" * 64
        raw = canonical_json(value)
        publication.write_bytes(raw)
        publication.with_suffix(".sha256").write_bytes(f"{sha256_bytes(raw)}  observation.json\n".encode())
        with self.assertRaises(PreflightFailed):
            self.execute()
        self.assertEqual(self.launches, 1)

    def test_incomplete_containment_preserves_environment(self):
        self.failure, self.contained = True, False
        result = self.execute()
        self.assertEqual(result.state, "CLOSED")
        self.assertEqual(self.retire.call_count, 0)
        observation = self.transaction(result)["observation"]
        self.assertEqual(observation["cleanup_disposition"], "INCOMPLETE")
        self.assertFalse(observation["payload"]["acceptance_eligible"])

    def test_failed_retirement_is_recorded_truthfully(self):
        self.failure = self.failed_retirement = True
        result = self.execute()
        publication = next(self.remote_proof.glob("diagnostics/*/*/*/observation.json"))
        value = json.loads(publication.read_bytes())
        self.assertEqual(value["summary"]["failure"]["environment_retirement_disposition"], "failed")
        self.assertEqual(value["cleanup"], "INCOMPLETE")
        self.assertEqual(result.state, "CLOSED")
        self.assertEqual(len(list(publication.parent.glob("*.json"))), 2)  # intent + sole diagnostic
        self.assertEqual(len(list(publication.parent.glob("*.sha256"))), 1)

    def test_frozen_input_mismatch_refuses_before_reservation(self):
        cases = ((self.command, "source_mismatch", True),
                 (self.host["custody"]["artifact"], "id", 1),
                 (self.fixture, "identity_sha256", "0" * 64),
                 (self.common.verify_runner_identity.return_value, "launch_critical_manifest_sha256", "0" * 64))
        for target, key, value in cases:
            with self.subTest(key=key):
                old = target[key] if isinstance(target, dict) else getattr(target, key)
                if isinstance(target, dict):
                    target[key] = value
                else:
                    setattr(target, key, value)
                try:
                    with self.assertRaises(PreflightFailed):
                        self.execute()
                    self.assertFalse((self.base / "state").exists())
                    self.assertEqual(self.launches, 0)
                finally:
                    if isinstance(target, dict):
                        target[key] = old
                    else:
                        setattr(target, key, old)

    def test_corrupt_oversized_and_wrong_identity_observations_are_refused(self):
        self.execute()
        path = next(self.remote_proof.glob("diagnostics/*/*/*/observation.json"))
        sidecar = path.with_suffix(".sha256")
        original, original_sidecar = path.read_bytes(), sidecar.read_bytes()
        for mutation in ("sidecar", "noncanonical", "oversize", "worker_sha256", "adapter_source_commit", "reservation_identity", "host_manifest_sha256"):
            with self.subTest(mutation=mutation):
                raw = original
                value = json.loads(raw)
                if mutation in value["binding"]:
                    value["binding"][mutation] = "0" * len(value["binding"][mutation])
                    raw = canonical_json(value)
                elif mutation == "noncanonical":
                    raw = raw.rstrip(b"\n")
                elif mutation == "oversize":
                    raw = b"x" * (MAX_REMOTE_DOCUMENT_BYTES + 1)
                path.write_bytes(raw)
                sidecar.write_bytes(b"bad" if mutation == "sidecar" else f"{sha256_bytes(raw)}  observation.json\n".encode())
                with self.assertRaises(PreflightFailed):
                    self.execute()
                self.assertEqual(self.launches, 1)
                path.write_bytes(original)
                sidecar.write_bytes(original_sidecar)

    def test_guard_is_not_weakened_and_worker_bytes_are_verified(self):
        self.busy = True
        with self.assertRaises(PreflightFailed):
            self.execute()
        self.busy = False
        authority, delegation = write_authority(self.base)
        binding = self.runtime._request(json.loads(delegation), None)
        self.runtime._remote("inspect", binding)
        argv, data = self.local_worker.calls[-1]
        with self.assertRaises(AssertionError):
            self.local_worker.run(argv, input_bytes=data + b"# changed\n")

    def test_production_registry_and_live_authority_remain_disabled(self):
        self.assertEqual(set(PRODUCTION_ADAPTERS), {PLAN_ID})
        self.assertFalse(hasattr(PRODUCTION_ADAPTERS[PLAN_ID], "render_product_evidence"))
        authority = load_authority(ROOT / "CURRENT_SLICE.md")
        with self.assertRaisesRegex(PolicyError, "LIVE_EXECUTION_FORBIDDEN"):
            authorize_live_request(authority, LiveRequest(
                ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE, SOURCE_A, PLAN_ID, CAMPAIGN))
        result = subprocess.run([sys.executable, str(TOOLS / "proof-run.py"), "diagnose",
            "--source", SOURCE_A, "--plan", PLAN_ID, "--campaign", CAMPAIGN], capture_output=True)
        self.assertEqual(result.returncode, 2)
        self.assertIn(b"LIVE_EXECUTION_FORBIDDEN", result.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)

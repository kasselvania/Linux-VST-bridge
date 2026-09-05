#!/usr/bin/env python3
"""Deterministic and multiprocess tests for the PX2 transaction core."""

from __future__ import annotations

import multiprocessing
import os
import pathlib
import sys
import tempfile
import time
import unittest
import unittest.mock


TOOLS = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(TOOLS))

from classified_proof_backend import (  # noqa: E402
    AcceptancePlanAdapter,
    BackendError,
    BudgetExhausted,
    ClassifiedProofBackend,
    DiagnosticPlanAdapter,
    DurableStateError,
    FAILURE_MAX_BYTES,
    Observation,
    ObservationKind,
    OutcomeUnknown,
    PlanDescriptor,
    PreflightFailed,
    PRODUCTION_ADAPTERS,
    TransactionState,
    UnsupportedAdapter,
)
from proof_execution_policy import (  # noqa: E402
    ExecutionClass,
    LiveRequest,
    authorize_live_request,
    canonical_json,
    load_authority,
    parse_canonical_json,
    sha256_bytes,
)


CONTRACT_BYTES = b"pc0-selection-v2 frozen product contract\n"
CONTRACT_ID = "pc0-selection-v2"
DIAGNOSTIC_PLAN = "px2-diagnostic-test"
ACCEPTANCE_PLAN = "px2-acceptance-test"
SOURCE_A = "1" * 40
SOURCE_B = "2" * 40
SOURCE_C = "3" * 40
IDENTITY = "4" * 64


def descriptor(
    execution_class: ExecutionClass,
    *,
    plan_id: str | None = None,
) -> PlanDescriptor:
    return PlanDescriptor.create(
        plan_id=plan_id or (
            DIAGNOSTIC_PLAN
            if execution_class is ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE
            else ACCEPTANCE_PLAN
        ),
        execution_class=execution_class,
        product_contract_identity=CONTRACT_ID,
        product_contract_bytes=CONTRACT_BYTES,
        operation="injected-local-test",
        artifact_requirement={"kind": "none"},
        fixture_requirement={"kind": "none"},
        runtime_requirement={"kind": "python-local"},
    )


def authority_text(
    plan: PlanDescriptor,
    *,
    source: str,
    identity: str,
    budget: int,
) -> str:
    if plan.execution_class is ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE:
        fields = {
            "status": "active_diagnostic_campaign",
            "authority_phase": "proof_harness_maintenance",
            "change_class": "PROOF_HARNESS_MAINTENANCE",
            "product_implementation_authorized": "false",
            "live_execution_authorized": "true",
            "permitted_execution_class": plan.execution_class.value,
            "classified_backend_core_ready": "true",
            "classified_backend_ready": "true",
            "diagnostic_campaign_identity": identity,
            "diagnostic_batch_budget": str(budget),
        }
    else:
        fields = {
            "status": "active_acceptance_candidate",
            "authority_phase": "implementation",
            "change_class": "PRODUCT_CONTRACT_CHANGE",
            "product_implementation_authorized": "true",
            "live_execution_authorized": "true",
            "permitted_execution_class": plan.execution_class.value,
            "classified_backend_core_ready": "true",
            "classified_backend_ready": "true",
            "acceptance_candidate_identity": identity,
            "acceptance_batch_budget": str(budget),
        }
    fields.update({
        "authorized_source_commit": source,
        "authorized_plan_id": plan.plan_id,
        "authorized_product_contract_identity": plan.product_contract_identity,
        "authorized_product_contract_sha256": plan.product_contract_sha256,
        "authorized_plan_content_sha256": plan.plan_content_sha256,
    })
    body = "\n".join(f"{key}: {value}" for key, value in fields.items())
    return f"# Test Authority\n\n## Authority\n\n```yaml\n{body}\n```\n"


def write_authority(
    directory: pathlib.Path,
    plan: PlanDescriptor,
    *,
    source: str = SOURCE_A,
    identity: str = IDENTITY,
    budget: int = 1,
    name: str = "authority.md",
):
    path = directory / name
    path.write_text(
        authority_text(plan, source=source, identity=identity, budget=budget),
        encoding="utf-8",
    )
    authority = load_authority(path)
    value = authorize_live_request(
        authority,
        LiveRequest(plan.execution_class, source, plan.plan_id, identity),
    )
    return authority, canonical_json(value)


def successful_observation() -> Observation:
    return Observation(
        kind=ObservationKind.SUCCESS,
        classification="INJECTED_SUCCESS",
        payload={"value": "retained"},
        cleanup_disposition="COMPLETE",
        protected_state_disposition="UNCHANGED",
        effects={"workload_launches": 1},
    )


class Behavior:
    def __init__(self, observation: Observation | None = None) -> None:
        self.observation = observation or successful_observation()
        self.reconciled: Observation | None = None
        self.preflight_error = False
        self.invoke_mode = "return"
        self.preflight_calls = 0
        self.reconcile_calls = 0
        self.invoke_calls = 0
        self.admit_calls = 0
        self.render_calls = 0
        self.preflight_probe = None

    def preflight(self, _context) -> None:
        self.preflight_calls += 1
        if self.preflight_probe is not None:
            self.preflight_probe()
        if self.preflight_error:
            raise RuntimeError("injected preflight refusal")

    def reconcile(self, _context) -> Observation | None:
        self.reconcile_calls += 1
        return self.reconciled

    def invoke(self, _context) -> Observation:
        self.invoke_calls += 1
        if self.invoke_mode == "unknown":
            raise OutcomeUnknown(
                effects={"driver_invocations": 1, "workload_launches": "unknown"},
                cleanup_disposition="UNKNOWN",
                protected_state_disposition="UNKNOWN",
            )
        return self.observation

    def admit(self, _context, observation: Observation):
        self.admit_calls += 1
        return {"admitted": True, "classification": observation.classification}

    def render(self, _context, _result) -> None:
        self.render_calls += 1


def diagnostic_adapter(plan: PlanDescriptor, behavior: Behavior) -> DiagnosticPlanAdapter:
    return DiagnosticPlanAdapter(
        descriptor=plan,
        preflight=behavior.preflight,
        reconcile=behavior.reconcile,
        invoke=behavior.invoke,
        admit=behavior.admit,
    )


def acceptance_adapter(plan: PlanDescriptor, behavior: Behavior) -> AcceptancePlanAdapter:
    return AcceptancePlanAdapter(
        descriptor=plan,
        preflight=behavior.preflight,
        reconcile=behavior.reconcile,
        invoke=behavior.invoke,
        admit=behavior.admit,
        render_product_evidence=behavior.render,
    )


class FileBehavior:
    def __init__(
        self,
        invoke_marker: pathlib.Path,
        *,
        delay: float = 0.0,
        crash: bool = False,
    ) -> None:
        self.invoke_marker = invoke_marker
        self.delay = delay
        self.crash = crash

    def preflight(self, _context) -> None:
        return None

    def reconcile(self, _context) -> Observation | None:
        return None

    def invoke(self, _context) -> Observation:
        descriptor_fd = os.open(
            self.invoke_marker, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600,
        )
        try:
            os.write(descriptor_fd, b"invoke\n")
            os.fsync(descriptor_fd)
        finally:
            os.close(descriptor_fd)
        if self.crash:
            os._exit(23)
        if self.delay:
            time.sleep(self.delay)
        return successful_observation()

    def admit(self, _context, observation: Observation):
        return {"classification": observation.classification}


def multiprocess_worker(
    state_root: str,
    authority_path: str,
    marker: str,
    queue,
    delay: float,
) -> None:
    plan = descriptor(ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE)
    authority = load_authority(pathlib.Path(authority_path))
    delegation = authorize_live_request(
        authority,
        LiveRequest(plan.execution_class, SOURCE_A, plan.plan_id, IDENTITY),
    )
    behavior = FileBehavior(pathlib.Path(marker), delay=delay)
    backend = ClassifiedProofBackend(
        pathlib.Path(state_root), {plan.plan_id: DiagnosticPlanAdapter(
            descriptor=plan,
            preflight=behavior.preflight,
            reconcile=behavior.reconcile,
            invoke=behavior.invoke,
            admit=behavior.admit,
        )},
    )
    try:
        receipt = backend.execute(authority, canonical_json(delegation))
        queue.put(("ok", receipt.state, receipt.budget_consumed))
    except Exception as exc:  # pragma: no cover - delivered to parent
        queue.put(("error", type(exc).__name__, str(exc)))


def crash_worker(state_root: str, authority_path: str, marker: str) -> None:
    plan = descriptor(ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE)
    authority = load_authority(pathlib.Path(authority_path))
    delegation = authorize_live_request(
        authority,
        LiveRequest(plan.execution_class, SOURCE_A, plan.plan_id, IDENTITY),
    )
    behavior = FileBehavior(pathlib.Path(marker), crash=True)
    backend = ClassifiedProofBackend(
        pathlib.Path(state_root), {plan.plan_id: DiagnosticPlanAdapter(
            descriptor=plan,
            preflight=behavior.preflight,
            reconcile=behavior.reconcile,
            invoke=behavior.invoke,
            admit=behavior.admit,
        )},
    )
    backend.execute(authority, canonical_json(delegation))


class BackendTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.base = pathlib.Path(self.temporary.name)
        self.state_root = self.base / "state"

    def test_combined_result_bound_recovers_observed_without_reexecution(self):
        from dataclasses import replace
        plan = descriptor(ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE)
        authority, delegation = write_authority(self.base, plan)
        observation = replace(successful_observation(), payload={'bounded_facts':'a'*(132*1024)})
        behavior = Behavior(observation)
        behavior.admit = lambda context, value: dict(value.payload)
        backend = ClassifiedProofBackend(self.state_root, {plan.plan_id:diagnostic_adapter(plan,behavior)})
        with unittest.mock.patch('classified_proof_backend.RESULT_ENVELOPE_MAX_BYTES',256*1024):
            with self.assertRaises(DurableStateError):backend.execute(authority,delegation)
        receipt=backend.execute(authority,delegation)
        self.assertEqual(receipt.state,'CLOSED');self.assertEqual(receipt.budget_consumed,1)
        self.assertEqual(behavior.invoke_calls,1);self.assertEqual(behavior.reconcile_calls,0)
        # Re-read and re-admit the larger envelope; neither component limit changed.
        backend.execute(authority,delegation);self.assertEqual(behavior.invoke_calls,1)
        with self.assertRaises(BackendError):replace(observation,payload={'bounded_facts':'a'*(257*1024)}).record()

    def test_production_registry_is_empty_and_unsupported_adapter_writes_nothing(self):
        self.assertEqual(dict(PRODUCTION_ADAPTERS), {})
        plan = descriptor(ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE)
        authority, delegation = write_authority(self.base, plan)
        backend = ClassifiedProofBackend(self.state_root)
        with self.assertRaises(UnsupportedAdapter):
            backend.execute(authority, delegation)
        self.assertFalse(self.state_root.exists())

    def test_failed_preflight_has_no_reachable_write_or_budget_path(self):
        plan = descriptor(ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE)
        authority, delegation = write_authority(self.base, plan)
        behavior = Behavior()
        behavior.preflight_error = True
        behavior.preflight_probe = lambda: self.assertFalse(self.state_root.exists())
        backend = ClassifiedProofBackend(
            self.state_root, {plan.plan_id: diagnostic_adapter(plan, behavior)},
        )
        with self.assertRaises(PreflightFailed):
            backend.execute(authority, delegation)
        self.assertEqual(behavior.invoke_calls, 0)
        self.assertFalse(self.state_root.exists())

    def test_stale_on_disk_authority_is_rejected_before_adapter_preflight(self):
        plan = descriptor(ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE)
        authority, delegation = write_authority(self.base, plan)
        behavior = Behavior()
        authority.path.write_text(
            authority_text(plan, source=SOURCE_B, identity=IDENTITY, budget=1),
            encoding="utf-8",
        )
        backend = ClassifiedProofBackend(
            self.state_root, {plan.plan_id: diagnostic_adapter(plan, behavior)},
        )
        with self.assertRaisesRegex(BackendError, "current on-disk authority"):
            backend.execute(authority, delegation)
        self.assertEqual(behavior.preflight_calls, 0)
        self.assertEqual(behavior.invoke_calls, 0)
        self.assertFalse(self.state_root.exists())

    def test_authority_drift_before_reservation_creates_no_transaction_or_budget(self):
        plan = descriptor(ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE)
        for phase in ("preflight", "under_budget_lock"):
            with self.subTest(phase=phase):
                state_root = self.base / f"state-{phase}"
                authority, delegation = write_authority(
                    self.base, plan, name=f"authority-{phase}.md",
                )
                behavior = Behavior()
                backend = ClassifiedProofBackend(
                    state_root, {plan.plan_id: diagnostic_adapter(plan, behavior)},
                )

                def mutate_authority():
                    authority.path.write_text(
                        authority_text(
                            plan, source=SOURCE_B, identity=IDENTITY, budget=1,
                        ),
                        encoding="utf-8",
                    )

                if phase == "preflight":
                    behavior.preflight_probe = mutate_authority
                    with self.assertRaisesRegex(BackendError, "current on-disk authority"):
                        backend.execute(authority, delegation)
                    self.assertFalse(state_root.exists())
                else:
                    load_budget = backend._load_or_initialize_budget

                    def mutate_after_lock(paths, admitted_delegation, admitted_authority=None):
                        budget = load_budget(paths, admitted_delegation, admitted_authority)
                        mutate_authority()
                        return budget

                    with unittest.mock.patch.object(
                        backend, "_load_or_initialize_budget", side_effect=mutate_after_lock,
                    ):
                        with self.assertRaisesRegex(BackendError, "current on-disk authority"):
                            backend.execute(authority, delegation)
                    self.assertEqual(
                        {path.name for path in state_root.rglob("*") if path.is_file()},
                        {"budget.lock"},
                    )
                self.assertEqual(behavior.preflight_calls, 1)
                self.assertEqual(behavior.invoke_calls, 0)
                self.assertEqual(list(state_root.rglob("transaction.json")), [])
                self.assertEqual(list(state_root.rglob("budget.json")), [])

    def test_campaign_budget_survives_source_revisions_and_refuses_third_batch(self):
        plan = descriptor(ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE)
        behavior = Behavior()
        backend = ClassifiedProofBackend(
            self.state_root, {plan.plan_id: diagnostic_adapter(plan, behavior)},
        )
        authorities = [
            write_authority(
                self.base, plan, source=source, budget=2,
                name=f"authority-{index}.md",
            )
            for index, source in enumerate((SOURCE_A, SOURCE_B, SOURCE_C), 1)
        ]
        first = backend.execute(*authorities[0])
        second = backend.execute(*authorities[1])
        self.assertEqual(first.budget_consumed, 1)
        self.assertEqual(second.budget_consumed, 2)
        with self.assertRaises(BudgetExhausted):
            backend.execute(*authorities[2])
        self.assertEqual(behavior.invoke_calls, 2)

    def test_extension_preserves_two_consumed_and_unknown_history_with_six_left(self):
        plan = descriptor(ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE)
        behavior = Behavior()
        backend = ClassifiedProofBackend(self.state_root, {plan.plan_id: diagnostic_adapter(plan, behavior)})
        first = backend.execute(*write_authority(self.base, plan, source=SOURCE_A, budget=2))
        behavior.invoke_mode = "unknown"
        second = backend.execute(*write_authority(self.base, plan, source=SOURCE_B, budget=2))
        self.assertEqual(second.state, "OUTCOME_UNKNOWN")
        campaign = self.state_root / "diagnostic" / IDENTITY
        old_files = {p:p.read_bytes() for p in (campaign / "transactions").rglob("*") if p.is_file()}
        old_budget = parse_canonical_json((campaign / "budget.json").read_bytes())
        original_delegations = []
        def reconcile(context):
            original_delegations.append(context.delegation)
            return None
        behavior.reconcile = reconcile
        backend = ClassifiedProofBackend(self.state_root, {plan.plan_id: diagnostic_adapter(plan, behavior)})
        # Extension itself reconciles the old reservation; no new invocation.
        enlarged = write_authority(self.base, plan, source=SOURCE_B, budget=8)
        extended = backend.execute(*enlarged)
        repeated = backend.execute(*enlarged)
        self.assertEqual((extended.budget_consumed, repeated.budget_consumed), (2, 2))
        self.assertEqual(behavior.invoke_calls, 2)
        self.assertTrue(all(d["batch_budget_maximum"] == 2 for d in original_delegations))
        self.assertEqual({p:p.read_bytes() for p in old_files}, old_files)
        adjusted = parse_canonical_json((campaign / "budget.json").read_bytes())
        self.assertEqual(adjusted, {**old_budget, "batch_budget_maximum":8})
        adjustment = parse_canonical_json((campaign / "budget-adjustment-2-to-8.json").read_bytes())
        self.assertEqual(adjustment["before"], old_budget)
        self.assertEqual(adjustment["after"], adjusted)
        self.assertEqual(adjustment["authority_sha256"], enlarged[0].raw_sha256)
        behavior.invoke_mode = "return"
        for n in range(3, 9):
            result = backend.execute(*write_authority(self.base, plan, source=f"{n:040x}", budget=8))
            self.assertEqual(result.budget_consumed, n)
        with self.assertRaises(BudgetExhausted):
            backend.execute(*write_authority(self.base, plan, source=f"{9:040x}", budget=8))
        self.assertEqual(behavior.invoke_calls, 8)
        self.assertEqual({p:p.read_bytes() for p in old_files}, old_files)

    def test_exact_diagnostic_plan_revision_preserves_budget_and_history(self):
        plan = descriptor(ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE)
        behavior = Behavior()
        backend = ClassifiedProofBackend(self.state_root, {plan.plan_id: diagnostic_adapter(plan, behavior)})
        for source in (SOURCE_A, SOURCE_B):
            backend.execute(*write_authority(self.base, plan, source=source, budget=8))
        root = self.state_root / "diagnostic" / IDENTITY
        old = parse_canonical_json((root / "budget.json").read_bytes())
        history = {p: p.read_bytes() for p in (root / "transactions").rglob("*") if p.is_file()}
        new_plan = PlanDescriptor.create(
            plan_id=plan.plan_id, execution_class=plan.execution_class,
            product_contract_identity=plan.product_contract_identity,
            product_contract_bytes=b"corrected notification semantics",
            operation=plan.plan_content["operation"],
            artifact_requirement={"kind":"corrected-host"},
            fixture_requirement=plan.plan_content["fixture_requirement"],
            runtime_requirement=plan.plan_content["runtime_requirement"])
        backend = ClassifiedProofBackend(self.state_root, {plan.plan_id: diagnostic_adapter(new_plan, behavior)})
        auth, delegation = write_authority(self.base, new_plan, source=SOURCE_C, budget=8)
        with self.assertRaisesRegex(DurableStateError, "unauthorized plan revision"):
            backend.execute(auth, delegation)
        text = auth.path.read_text()
        additions = ("diagnostic_plan_revision_authorized: true\n"
            f"diagnostic_previous_plan_content_sha256: {plan.plan_content_sha256}\n"
            f"diagnostic_previous_product_contract_sha256: {plan.product_contract_sha256}\n")
        auth.path.write_text(text.replace("status:", additions + "status:", 1))
        valid_text = auth.path.read_text()
        auth.path.write_text(valid_text.replace(plan.plan_content_sha256, "f" * 64))
        wrong = load_authority(auth.path)
        wrong_delegation = canonical_json(authorize_live_request(wrong, LiveRequest(new_plan.execution_class, SOURCE_C, new_plan.plan_id, IDENTITY)))
        with self.assertRaisesRegex(DurableStateError, "unauthorized plan revision"):
            backend.execute(wrong, wrong_delegation)
        self.assertEqual((root / "budget.json").read_bytes(), canonical_json(old))
        auth.path.write_text(valid_text)
        auth = load_authority(auth.path)
        delegation = canonical_json(authorize_live_request(auth, LiveRequest(new_plan.execution_class, SOURCE_C, new_plan.plan_id, IDENTITY)))
        for _ in range(2):
            receipt = backend.execute(auth, delegation)
            self.assertEqual(receipt.budget_consumed, 3)
        budget = parse_canonical_json((root / "budget.json").read_bytes())
        self.assertEqual(budget["reservations"][:2], old["reservations"])
        self.assertEqual(budget["batch_budget_maximum"], 8)
        self.assertEqual({p:p.read_bytes() for p in history}, history)
        adjustment = parse_canonical_json(next(root.glob("plan-adjustment-*.json")).read_bytes())
        self.assertEqual(adjustment["before"], old)
        self.assertEqual(adjustment["after"]["consumed_count"], 2)
        self.assertEqual(behavior.invoke_calls, 3)
        for n in range(4, 9):
            backend.execute(*write_authority(self.base, new_plan, source=f"{n:040x}", budget=8))
        with self.assertRaises(BudgetExhausted):
            backend.execute(*write_authority(self.base, new_plan, source=f"{9:040x}", budget=8))

    def test_extension_rejects_corruption_stable_drift_decrease_and_stale_authority(self):
        plan = descriptor(ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE)
        behavior = Behavior()
        backend = ClassifiedProofBackend(self.state_root, {plan.plan_id: diagnostic_adapter(plan, behavior)})
        backend.execute(*write_authority(self.base, plan, budget=2))
        path = self.state_root / "diagnostic" / IDENTITY / "budget.json"
        original = path.read_bytes()
        authority = write_authority(self.base, plan, source=SOURCE_B, budget=8)
        mutations = {"batch_budget_maximum": [True, "2", 0, -1, 1.5],
                     "consumed_count": [0, True, 2], "reservations": [[], ["bad"]],
                     "execution_identity": ["e"*64], "plan_id": ["wrong"],
                     "product_contract_sha256": ["a"*64]}
        for key, values in mutations.items():
            for value in values:
                with self.subTest(key=key, value=value):
                    budget = parse_canonical_json(original)
                    budget[key] = value
                    path.write_bytes(canonical_json(budget))
                    with self.assertRaises(DurableStateError):
                        backend.execute(*authority)
                    self.assertEqual(behavior.invoke_calls, 1)
                    self.assertFalse(path.with_name("budget-adjustment-2-to-8.json").exists())
        path.write_bytes(original)
        with self.assertRaises(DurableStateError):
            backend.execute(*write_authority(self.base, plan, source=SOURCE_B, budget=1))
        stale = write_authority(self.base, plan, source=SOURCE_B, budget=8)
        stale[0].path.write_text(authority_text(plan, source=SOURCE_C, identity=IDENTITY, budget=8))
        with self.assertRaises(Exception):
            backend.execute(*stale)
        self.assertEqual(path.read_bytes(), original)
        self.assertFalse(path.with_name("budget-adjustment-2-to-8.json").exists())

    def test_acceptance_candidate_reserves_once_and_source_is_frozen(self):
        plan = descriptor(ExecutionClass.ACCEPTANCE_CANDIDATE)
        behavior = Behavior()
        backend = ClassifiedProofBackend(
            self.state_root, {plan.plan_id: acceptance_adapter(plan, behavior)},
        )
        authority_a = write_authority(self.base, plan, name="accept-a.md")
        first = backend.execute(*authority_a)
        second = backend.execute(*authority_a)
        self.assertEqual(first.budget_consumed, 1)
        self.assertEqual(second.budget_consumed, 1)
        self.assertEqual(behavior.invoke_calls, 1)
        self.assertEqual(behavior.render_calls, 1)

        authority_b = write_authority(
            self.base, plan, source=SOURCE_B, name="accept-b.md",
        )
        with self.assertRaises(DurableStateError):
            backend.execute(*authority_b)
        self.assertEqual(behavior.invoke_calls, 1)

        changed_plan = PlanDescriptor.create(
            plan_id=plan.plan_id,
            execution_class=ExecutionClass.ACCEPTANCE_CANDIDATE,
            product_contract_identity=CONTRACT_ID,
            product_contract_bytes=CONTRACT_BYTES,
            operation="injected-changed-plan",
            artifact_requirement={"kind": "none"},
            fixture_requirement={"kind": "none"},
            runtime_requirement={"kind": "python-local"},
        )
        changed_behavior = Behavior()
        changed_authority = write_authority(
            self.base, changed_plan, name="accept-changed-plan.md",
        )
        with self.assertRaises(DurableStateError):
            ClassifiedProofBackend(
                self.state_root,
                {changed_plan.plan_id: acceptance_adapter(
                    changed_plan, changed_behavior,
                )},
            ).execute(*changed_authority)
        self.assertEqual(changed_behavior.invoke_calls, 0)

    def test_diagnostic_and_acceptance_budgets_are_independent(self):
        diagnostic = descriptor(ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE)
        acceptance = descriptor(ExecutionClass.ACCEPTANCE_CANDIDATE)
        diagnostic_behavior = Behavior()
        acceptance_behavior = Behavior()
        adapters = {
            diagnostic.plan_id: diagnostic_adapter(diagnostic, diagnostic_behavior),
            acceptance.plan_id: acceptance_adapter(acceptance, acceptance_behavior),
        }
        backend = ClassifiedProofBackend(self.state_root, adapters)
        diagnostic_authority = write_authority(
            self.base, diagnostic, name="diagnostic.md",
        )
        acceptance_authority = write_authority(
            self.base, acceptance, name="acceptance.md",
        )
        diagnostic_receipt = backend.execute(*diagnostic_authority)
        acceptance_receipt = backend.execute(*acceptance_authority)
        self.assertEqual(diagnostic_receipt.budget_consumed, 1)
        self.assertEqual(acceptance_receipt.budget_consumed, 1)
        self.assertEqual(diagnostic_behavior.invoke_calls, 1)
        self.assertEqual(acceptance_behavior.invoke_calls, 1)

    def test_competing_processes_create_exactly_one_reservation_and_launch(self):
        plan = descriptor(ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE)
        authority_path = self.base / "authority.md"
        authority_path.write_text(
            authority_text(plan, source=SOURCE_A, identity=IDENTITY, budget=1),
            encoding="utf-8",
        )
        marker = self.base / "invocations.log"
        context = multiprocessing.get_context("fork")
        queue = context.Queue()
        processes = [
            context.Process(
                target=multiprocess_worker,
                args=(str(self.state_root), str(authority_path), str(marker), queue, 0.2),
            )
            for _ in range(2)
        ]
        for process in processes:
            process.start()
        for process in processes:
            process.join(10)
            self.assertFalse(process.is_alive())
            self.assertEqual(process.exitcode, 0)
        results = [queue.get(timeout=2) for _ in processes]
        self.assertTrue(all(item[0] == "ok" for item in results), results)
        self.assertEqual(marker.read_text(encoding="utf-8").splitlines(), ["invoke"])
        budget = parse_canonical_json(
            (self.state_root / "diagnostic" / IDENTITY / "budget.json").read_bytes(),
            maximum=512 * 1024,
        )
        self.assertEqual(budget["consumed_count"], 1)
        self.assertEqual(len(budget["reservations"]), 1)

    def test_crash_after_reservation_consumes_batch_and_never_relaunches(self):
        plan = descriptor(ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE)
        authority_path = self.base / "authority.md"
        authority_path.write_text(
            authority_text(plan, source=SOURCE_A, identity=IDENTITY, budget=1),
            encoding="utf-8",
        )
        marker = self.base / "crash-invocations.log"
        context = multiprocessing.get_context("fork")
        process = context.Process(
            target=crash_worker,
            args=(str(self.state_root), str(authority_path), str(marker)),
        )
        process.start()
        process.join(10)
        self.assertEqual(process.exitcode, 23)

        budget = parse_canonical_json(
            (self.state_root / "diagnostic" / IDENTITY / "budget.json").read_bytes(),
            maximum=512 * 1024,
        )
        self.assertEqual(budget["consumed_count"], 1)
        reservation_identity = budget["reservations"][0]
        transaction = parse_canonical_json(
            (self.state_root / "diagnostic" / IDENTITY / "transactions"
             / reservation_identity / "transaction.json").read_bytes(),
            maximum=512 * 1024,
        )
        self.assertEqual(transaction["state"], TransactionState.LAUNCHING.value)
        self.assertEqual(transaction["delegation"]["source_commit"], SOURCE_A)
        self.assertEqual(
            transaction["plan_descriptor"]["plan_content_sha256"],
            plan.plan_content_sha256,
        )

        authority = load_authority(authority_path)
        delegation = authorize_live_request(
            authority,
            LiveRequest(plan.execution_class, SOURCE_A, plan.plan_id, IDENTITY),
        )
        behavior = Behavior()
        backend = ClassifiedProofBackend(
            self.state_root, {plan.plan_id: diagnostic_adapter(plan, behavior)},
        )
        receipt = backend.execute(authority, canonical_json(delegation))
        self.assertEqual(receipt.state, TransactionState.OUTCOME_UNKNOWN.value)
        self.assertEqual(receipt.budget_consumed, 1)
        self.assertEqual(behavior.invoke_calls, 0)
        self.assertEqual(marker.read_text(encoding="utf-8").splitlines(), ["invoke"])

    def test_lost_acknowledgement_reconciles_without_relaunch(self):
        plan = descriptor(ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE)
        authority, delegation = write_authority(self.base, plan)
        behavior = Behavior()
        behavior.invoke_mode = "unknown"
        behavior.reconciled = successful_observation()
        backend = ClassifiedProofBackend(
            self.state_root, {plan.plan_id: diagnostic_adapter(plan, behavior)},
        )
        receipt = backend.execute(authority, delegation)
        self.assertEqual(receipt.state, TransactionState.CLOSED.value)
        self.assertIsNotNone(receipt.result_sha256)
        self.assertEqual(behavior.invoke_calls, 1)
        backend.execute(authority, delegation)
        self.assertEqual(behavior.invoke_calls, 1)

    def test_unknown_outcome_can_later_reconcile_but_never_relaunch(self):
        plan = descriptor(ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE)
        authority, delegation = write_authority(self.base, plan)
        behavior = Behavior()
        behavior.invoke_mode = "unknown"
        backend = ClassifiedProofBackend(
            self.state_root, {plan.plan_id: diagnostic_adapter(plan, behavior)},
        )
        unknown = backend.execute(authority, delegation)
        self.assertEqual(unknown.state, TransactionState.OUTCOME_UNKNOWN.value)
        behavior.reconciled = successful_observation()
        recovered = backend.execute(authority, delegation)
        self.assertEqual(recovered.state, TransactionState.CLOSED.value)
        self.assertIsNotNone(recovered.result_sha256)
        self.assertEqual(behavior.invoke_calls, 1)

    def test_retained_result_is_readmitted_without_reexecution(self):
        plan = descriptor(ExecutionClass.ACCEPTANCE_CANDIDATE)
        authority, delegation = write_authority(self.base, plan)
        behavior = Behavior()
        backend = ClassifiedProofBackend(
            self.state_root, {plan.plan_id: acceptance_adapter(plan, behavior)},
        )
        first = backend.execute(authority, delegation)
        second = backend.execute(authority, delegation)
        self.assertEqual(first.result_sha256, second.result_sha256)
        self.assertEqual(behavior.invoke_calls, 1)
        self.assertEqual(behavior.admit_calls, 2)
        self.assertEqual(behavior.render_calls, 1)

    def test_renderer_is_structurally_unreachable_for_nonaccepted_outcomes(self):
        diagnostic_plan = descriptor(ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE)
        diagnostic_behavior = Behavior()
        diagnostic = diagnostic_adapter(diagnostic_plan, diagnostic_behavior)
        self.assertFalse(hasattr(diagnostic, "render_product_evidence"))
        authority, delegation = write_authority(
            self.base, diagnostic_plan, name="diagnostic.md",
        )
        ClassifiedProofBackend(
            self.state_root / "diagnostic", {diagnostic_plan.plan_id: diagnostic},
        ).execute(authority, delegation)
        self.assertEqual(diagnostic_behavior.render_calls, 0)

        for index, kind in enumerate(
            (ObservationKind.FAILED, ObservationKind.INCONCLUSIVE), 1,
        ):
            with self.subTest(kind=kind.value):
                plan = descriptor(ExecutionClass.ACCEPTANCE_CANDIDATE)
                behavior = Behavior(Observation(
                    kind=kind,
                    classification=f"INJECTED_{kind.value}",
                    payload={},
                    cleanup_disposition="COMPLETE",
                    protected_state_disposition="UNCHANGED",
                    effects={"workload_launches": 1},
                ))
                identity = str(index + 5) * 64
                authority, delegation = write_authority(
                    self.base, plan, identity=identity,
                    name=f"acceptance-{index}.md",
                )
                receipt = ClassifiedProofBackend(
                    self.state_root / f"acceptance-{index}",
                    {plan.plan_id: acceptance_adapter(plan, behavior)},
                ).execute(authority, delegation)
                self.assertEqual(receipt.state, TransactionState.CLOSED.value)
                self.assertIsNone(receipt.result_sha256)
                self.assertEqual(behavior.render_calls, 0)

        success_plan = descriptor(ExecutionClass.ACCEPTANCE_CANDIDATE)
        success_behavior = Behavior()
        success_authority = write_authority(
            self.base, success_plan, identity="8" * 64, name="success.md",
        )
        success_receipt = ClassifiedProofBackend(
            self.state_root / "success",
            {success_plan.plan_id: acceptance_adapter(success_plan, success_behavior)},
        ).execute(*success_authority)
        self.assertEqual(
            success_receipt.state, TransactionState.ACCEPTANCE_EVIDENCE_RENDERED.value,
        )
        self.assertTrue(success_receipt.renderer_completed)
        self.assertEqual(success_behavior.render_calls, 1)

    def test_failure_pair_is_canonical_bounded_and_preserves_unknown_effects(self):
        plan = descriptor(ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE)
        authority, delegation = write_authority(self.base, plan)
        behavior = Behavior()
        behavior.invoke_mode = "unknown"
        backend = ClassifiedProofBackend(
            self.state_root, {plan.plan_id: diagnostic_adapter(plan, behavior)},
        )
        receipt = backend.execute(authority, delegation)
        transaction_dir = (
            self.state_root / "diagnostic" / IDENTITY / "transactions"
            / receipt.reservation_identity
        )
        failure_path = transaction_dir / "classified-failure.json"
        sidecar_path = transaction_dir / "classified-failure.json.sha256"
        raw = failure_path.read_bytes()
        self.assertLessEqual(len(raw), FAILURE_MAX_BYTES)
        failure = parse_canonical_json(raw, maximum=FAILURE_MAX_BYTES)
        self.assertEqual(canonical_json(failure), raw)
        self.assertEqual(failure["effects"]["workload_launches"], "unknown")
        self.assertEqual(failure["effects"]["driver_invocations"], 1)
        digest = sha256_bytes(raw)
        self.assertEqual(
            sidecar_path.read_bytes(),
            f"{digest}  classified-failure.json\n".encode("ascii"),
        )
        self.assertEqual(receipt.failure_sha256, digest)
        sidecar_path.write_text("0" * 64 + "  classified-failure.json\n",
                                encoding="ascii")
        with self.assertRaises(DurableStateError):
            backend.execute(authority, delegation)
        self.assertEqual(behavior.invoke_calls, 1)

    def test_tampered_plan_descriptor_and_delegation_fail_before_preflight(self):
        plan = descriptor(ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE)
        authority, delegation = write_authority(self.base, plan)
        behavior = Behavior()
        tampered_plan = PlanDescriptor(
            plan_id=plan.plan_id,
            execution_class=plan.execution_class,
            product_contract_identity=plan.product_contract_identity,
            product_contract_sha256=plan.product_contract_sha256,
            plan_content=plan.plan_content,
            plan_content_sha256="9" * 64,
        )
        backend = ClassifiedProofBackend(
            self.state_root,
            {plan.plan_id: diagnostic_adapter(tampered_plan, behavior)},
        )
        with self.assertRaises(BackendError):
            backend.execute(authority, delegation)
        self.assertEqual(behavior.preflight_calls, 0)
        self.assertFalse(self.state_root.exists())

        noncanonical = delegation.rstrip(b"\n")
        backend = ClassifiedProofBackend(
            self.state_root, {plan.plan_id: diagnostic_adapter(plan, behavior)},
        )
        with self.assertRaises(BackendError):
            backend.execute(authority, noncanonical)
        self.assertEqual(behavior.preflight_calls, 0)
        self.assertFalse(self.state_root.exists())


if __name__ == "__main__":
    unittest.main(verbosity=2)

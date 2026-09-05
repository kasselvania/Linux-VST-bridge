#!/usr/bin/env python3
"""PX2 class-aware proof transaction core.

The core is deliberately generic and has no production adapters. It writes
canonical files through a same-directory temporary file, fsyncs the file,
atomically replaces the destination, and fsyncs the parent directory when the
host filesystem supports directory fsync. A POSIX ``flock`` held for the whole
reservation/invocation/closure sequence provides cross-process single-writer
ownership. These guarantees do not imply atomicity across multiple files; the
reader reconciles every partial ordering conservatively as consumed.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
import errno
import fcntl
import json
import os
import pathlib
import re
from types import MappingProxyType
from typing import Any, Callable, Mapping, TypeAlias
import uuid

from proof_execution_policy import (
    Authority,
    ExecutionClass,
    HEX64,
    IDENTIFIER,
    PolicyError,
    canonical_json,
    load_authority,
    parse_canonical_json,
    sha256_bytes,
    validate_canonical_delegation,
)


PLAN_CONTENT_SCHEMA = "linux-vst-bridge-closed-proof-plan/v1"
PLAN_DESCRIPTOR_SCHEMA = "linux-vst-bridge-proof-plan-descriptor/v1"
BUDGET_SCHEMA = "linux-vst-bridge-classified-proof-budget/v1"
TRANSACTION_SCHEMA = "linux-vst-bridge-classified-proof-transaction/v1"
RESULT_SCHEMA = "linux-vst-bridge-classified-proof-result/v1"
FAILURE_SCHEMA = "linux-vst-bridge-classified-proof-failure/v1"
RESERVATION_SCHEMA = "linux-vst-bridge-proof-reservation-identity/v1"

FAILURE_MAX_BYTES = 64 * 1024
STATE_MAX_BYTES = 512 * 1024
PLAN_MAX_BYTES = 32 * 1024
RESULT_MAX_BYTES = 256 * 1024
# The envelope retains both the independently bounded observation and admission.
RESULT_ENVELOPE_MAX_BYTES = 512 * 1024
EFFECT_NAME = re.compile(r"[a-z][a-z0-9_]{0,63}")
CLASSIFICATION = re.compile(r"[A-Z][A-Z0-9_]{0,127}")


class BackendError(RuntimeError):
    """A fail-closed transaction-core refusal."""


class UnsupportedAdapter(BackendError):
    """The requested plan has no closed adapter."""


class PreflightFailed(BackendError):
    """A cheap read-only adapter preflight failed."""


class BudgetExhausted(BackendError):
    """The stable campaign or candidate has no unconsumed batch."""


class DurableStateError(BackendError):
    """Durable state was absent, malformed, or contradictory."""


class OutcomeUnknown(RuntimeError):
    """Invocation may have happened but acknowledgement is unavailable."""

    def __init__(
        self,
        *,
        effects: Mapping[str, int | str] | None = None,
        cleanup_disposition: str = "UNKNOWN",
        protected_state_disposition: str = "UNKNOWN",
    ) -> None:
        super().__init__("classified proof invocation outcome is unknown")
        self.effects = dict(effects or {"workload_launches": "unknown"})
        self.cleanup_disposition = cleanup_disposition
        self.protected_state_disposition = protected_state_disposition


class ObservationKind(str, Enum):
    SUCCESS = "SUCCESS"
    FAILED = "FAILED"
    INCONCLUSIVE = "INCONCLUSIVE"


class TransactionState(str, Enum):
    NEW = "NEW"
    PREFLIGHTED = "PREFLIGHTED"
    RESERVED = "RESERVED"
    LAUNCHING = "LAUNCHING"
    OBSERVED = "OBSERVED"
    OUTCOME_UNKNOWN = "OUTCOME_UNKNOWN"
    RESULT_RETAINED = "RESULT_RETAINED"
    FAILURE_RETAINED = "FAILURE_RETAINED"
    CLOSED = "CLOSED"
    ACCEPTANCE_EVIDENCE_RENDERED = "ACCEPTANCE_EVIDENCE_RENDERED"


TRANSITIONS = {
    TransactionState.NEW: {TransactionState.PREFLIGHTED},
    TransactionState.PREFLIGHTED: {
        TransactionState.RESERVED, TransactionState.OUTCOME_UNKNOWN,
    },
    TransactionState.RESERVED: {
        TransactionState.LAUNCHING, TransactionState.OBSERVED,
        TransactionState.OUTCOME_UNKNOWN,
    },
    TransactionState.LAUNCHING: {
        TransactionState.OBSERVED, TransactionState.OUTCOME_UNKNOWN,
    },
    TransactionState.OUTCOME_UNKNOWN: {TransactionState.OBSERVED},
    TransactionState.OBSERVED: {
        TransactionState.RESULT_RETAINED, TransactionState.FAILURE_RETAINED,
    },
    TransactionState.RESULT_RETAINED: {TransactionState.CLOSED},
    TransactionState.FAILURE_RETAINED: {TransactionState.CLOSED},
    TransactionState.CLOSED: {TransactionState.ACCEPTANCE_EVIDENCE_RENDERED},
    TransactionState.ACCEPTANCE_EVIDENCE_RENDERED: set(),
}


@dataclass(frozen=True, slots=True)
class PlanDescriptor:
    plan_id: str
    execution_class: ExecutionClass
    product_contract_identity: str
    product_contract_sha256: str
    plan_content: Mapping[str, Any]
    plan_content_sha256: str

    @classmethod
    def create(
        cls,
        *,
        plan_id: str,
        execution_class: ExecutionClass,
        product_contract_identity: str,
        product_contract_bytes: bytes,
        operation: str,
        artifact_requirement: Mapping[str, Any],
        fixture_requirement: Mapping[str, Any],
        runtime_requirement: Mapping[str, Any],
    ) -> "PlanDescriptor":
        if not isinstance(product_contract_bytes, bytes) or not product_contract_bytes:
            raise BackendError("product contract bytes are absent")
        if len(product_contract_bytes) > 256 * 1024:
            raise BackendError("product contract exceeds the descriptor bound")
        content = {
            "schema": PLAN_CONTENT_SCHEMA,
            "operation": operation,
            "artifact_requirement": dict(artifact_requirement),
            "fixture_requirement": dict(fixture_requirement),
            "runtime_requirement": dict(runtime_requirement),
        }
        content_bytes = canonical_json(content)
        if len(content_bytes) > PLAN_MAX_BYTES:
            raise BackendError("canonical plan content exceeds the bound")
        normalized = parse_canonical_json(content_bytes, maximum=PLAN_MAX_BYTES)
        descriptor = cls(
            plan_id=plan_id,
            execution_class=execution_class,
            product_contract_identity=product_contract_identity,
            product_contract_sha256=sha256_bytes(product_contract_bytes),
            plan_content=normalized,
            plan_content_sha256=sha256_bytes(content_bytes),
        )
        descriptor.validate()
        return descriptor

    def validate(self) -> None:
        if IDENTIFIER.fullmatch(self.plan_id) is None:
            raise BackendError("adapter plan ID is outside the closed syntax")
        if self.execution_class is ExecutionClass.READ_ONLY_RECONCILIATION:
            raise BackendError("read-only reconciliation cannot own a plan adapter")
        if IDENTIFIER.fullmatch(self.product_contract_identity) is None:
            raise BackendError("product contract identity is outside the closed syntax")
        if HEX64.fullmatch(self.product_contract_sha256) is None:
            raise BackendError("product contract digest is malformed")
        if not isinstance(self.plan_content, Mapping):
            raise BackendError("plan content must be one object")
        keys = {
            "schema", "operation", "artifact_requirement",
            "fixture_requirement", "runtime_requirement",
        }
        if set(self.plan_content) != keys:
            raise BackendError("plan content key roster differs")
        if self.plan_content.get("schema") != PLAN_CONTENT_SCHEMA:
            raise BackendError("plan content schema differs")
        if IDENTIFIER.fullmatch(str(self.plan_content.get("operation", ""))) is None:
            raise BackendError("plan operation is outside the closed syntax")
        for key in (
            "artifact_requirement", "fixture_requirement", "runtime_requirement",
        ):
            if not isinstance(self.plan_content.get(key), Mapping):
                raise BackendError(f"{key} must be one closed object")
        encoded = canonical_json(self.plan_content)
        if len(encoded) > PLAN_MAX_BYTES:
            raise BackendError("canonical plan content exceeds the bound")
        if sha256_bytes(encoded) != self.plan_content_sha256:
            raise BackendError("canonical plan-content digest differs")

    def record(self) -> dict[str, Any]:
        self.validate()
        return {
            "schema": PLAN_DESCRIPTOR_SCHEMA,
            "plan_id": self.plan_id,
            "execution_class": self.execution_class.value,
            "product_contract_identity": self.product_contract_identity,
            "product_contract_sha256": self.product_contract_sha256,
            "plan_content": json.loads(canonical_json(self.plan_content)),
            "plan_content_sha256": self.plan_content_sha256,
        }


@dataclass(frozen=True, slots=True)
class PreflightContext:
    delegation: Mapping[str, Any]
    plan_descriptor: Mapping[str, Any]


@dataclass(frozen=True, slots=True)
class ReservationContext:
    delegation: Mapping[str, Any]
    plan_descriptor: Mapping[str, Any]
    reservation_identity: str
    prior_transaction: Mapping[str, Any] | None


@dataclass(frozen=True, slots=True)
class Observation:
    kind: ObservationKind
    classification: str
    payload: Mapping[str, Any]
    cleanup_disposition: str
    protected_state_disposition: str
    effects: Mapping[str, int | str]

    def record(self) -> dict[str, Any]:
        if CLASSIFICATION.fullmatch(self.classification) is None:
            raise BackendError("observation classification is outside the closed syntax")
        if self.cleanup_disposition not in {"COMPLETE", "INCOMPLETE", "UNKNOWN"}:
            raise BackendError("cleanup disposition is outside the closed set")
        if self.protected_state_disposition not in {
            "UNCHANGED", "RESTORED", "CHANGED", "UNKNOWN",
        }:
            raise BackendError("protected-state disposition is outside the closed set")
        effects = _validated_effects(self.effects)
        if not isinstance(self.payload, Mapping):
            raise BackendError("observation payload must be one object")
        record = {
            "kind": self.kind.value,
            "classification": self.classification,
            "payload": dict(self.payload),
            "cleanup_disposition": self.cleanup_disposition,
            "protected_state_disposition": self.protected_state_disposition,
            "effects": effects,
        }
        raw = canonical_json(record)
        if len(raw) > RESULT_MAX_BYTES:
            raise BackendError("observation exceeds the retained-result bound")
        return parse_canonical_json(raw, maximum=RESULT_MAX_BYTES)

    @classmethod
    def from_record(cls, value: Mapping[str, Any]) -> "Observation":
        if not isinstance(value, Mapping) or set(value) != {
            "kind", "classification", "payload", "cleanup_disposition",
            "protected_state_disposition", "effects",
        }:
            raise DurableStateError("retained observation key roster differs")
        try:
            kind = ObservationKind(str(value["kind"]))
        except ValueError as exc:
            raise DurableStateError("retained observation kind differs") from exc
        observation = cls(
            kind=kind,
            classification=str(value["classification"]),
            payload=value["payload"],
            cleanup_disposition=str(value["cleanup_disposition"]),
            protected_state_disposition=str(value["protected_state_disposition"]),
            effects=value["effects"],
        )
        observation.record()
        return observation


Admit = Callable[[ReservationContext, Observation], Mapping[str, Any]]
Preflight = Callable[[PreflightContext], None]
Reconcile = Callable[[ReservationContext], Observation | None]
Invoke = Callable[[ReservationContext], Observation]
Render = Callable[[ReservationContext, Mapping[str, Any]], None]


@dataclass(frozen=True, slots=True)
class DiagnosticPlanAdapter:
    descriptor: PlanDescriptor
    preflight: Preflight
    reconcile: Reconcile
    invoke: Invoke
    admit: Admit


@dataclass(frozen=True, slots=True)
class AcceptancePlanAdapter:
    descriptor: PlanDescriptor
    preflight: Preflight
    reconcile: Reconcile
    invoke: Invoke
    admit: Admit
    render_product_evidence: Render


PlanAdapter: TypeAlias = DiagnosticPlanAdapter | AcceptancePlanAdapter


# PX2 intentionally ships no live adapter. Tests inject closed adapters.
PRODUCTION_ADAPTERS: Mapping[str, PlanAdapter] = MappingProxyType({})


@dataclass(frozen=True, slots=True)
class ExecutionReceipt:
    state: str
    reservation_identity: str
    execution_class: str
    budget_consumed: int
    budget_remaining: int
    result_sha256: str | None
    failure_sha256: str | None
    renderer_completed: bool


@dataclass(frozen=True, slots=True)
class _Paths:
    owner: pathlib.Path
    lock: pathlib.Path
    budget: pathlib.Path
    transaction_dir: pathlib.Path
    transaction: pathlib.Path
    result: pathlib.Path
    failure: pathlib.Path
    failure_sidecar: pathlib.Path


def _validated_effects(value: Mapping[str, int | str]) -> dict[str, int | str]:
    if not isinstance(value, Mapping) or len(value) > 64:
        raise BackendError("effect counts must be one bounded object")
    result: dict[str, int | str] = {}
    for key, item in value.items():
        if not isinstance(key, str) or EFFECT_NAME.fullmatch(key) is None:
            raise BackendError("effect-count name is outside the closed syntax")
        if not ((isinstance(item, int) and not isinstance(item, bool) and item >= 0)
                or item == "unknown"):
            raise BackendError("effect count must be nonnegative or 'unknown'")
        result[key] = item
    return result


def _fsync_directory(path: pathlib.Path) -> None:
    try:
        descriptor = os.open(path, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
    except OSError as exc:
        if exc.errno in {errno.EINVAL, errno.ENOTSUP, errno.EOPNOTSUPP}:
            return
        raise
    try:
        try:
            os.fsync(descriptor)
        except OSError as exc:
            if exc.errno not in {errno.EINVAL, errno.ENOTSUP, errno.EOPNOTSUPP}:
                raise
    finally:
        os.close(descriptor)


def _mkdir(path: pathlib.Path) -> None:
    if path.exists() and (not path.is_dir() or path.is_symlink()):
        raise DurableStateError(f"unsafe durable-state directory: {path.name}")
    if path.exists():
        return
    parent = path.parent
    if not parent.exists():
        _mkdir(parent)
    try:
        path.mkdir(mode=0o700)
    except FileExistsError:
        if not path.is_dir() or path.is_symlink():
            raise DurableStateError(f"unsafe durable-state directory: {path.name}")
    _fsync_directory(parent)


def _atomic_write(path: pathlib.Path, raw: bytes) -> None:
    _mkdir(path.parent)
    if path.is_symlink():
        raise DurableStateError(f"unsafe durable-state target: {path.name}")
    temporary = path.with_name(f".{path.name}.tmp-{os.getpid()}-{uuid.uuid4().hex}")
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL
    flags |= getattr(os, "O_CLOEXEC", 0)
    flags |= getattr(os, "O_NOFOLLOW", 0)
    descriptor = os.open(temporary, flags, 0o600)
    try:
        with os.fdopen(descriptor, "wb", closefd=False) as stream:
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        os.close(descriptor)
        descriptor = -1
        os.replace(temporary, path)
        _fsync_directory(path.parent)
    finally:
        if descriptor >= 0:
            os.close(descriptor)
        try:
            temporary.unlink()
        except FileNotFoundError:
            pass


def _read_object(path: pathlib.Path, *, maximum: int = STATE_MAX_BYTES) -> dict[str, Any]:
    if not path.is_file() or path.is_symlink():
        raise DurableStateError(f"durable state is absent or unsafe: {path.name}")
    if path.stat().st_size > maximum:
        raise DurableStateError(f"durable state exceeds its bound: {path.name}")
    try:
        value = parse_canonical_json(path.read_bytes(), maximum=maximum)
    except PolicyError as exc:
        raise DurableStateError(f"durable state is not canonical: {path.name}") from exc
    if not isinstance(value, dict):
        raise DurableStateError(f"durable state is not one object: {path.name}")
    return value


def _write_object(path: pathlib.Path, value: Mapping[str, Any], *, maximum: int) -> str:
    raw = canonical_json(value)
    if len(raw) > maximum:
        raise DurableStateError(f"durable object exceeds its bound: {path.name}")
    _atomic_write(path, raw)
    return sha256_bytes(raw)


class _BudgetLock:
    def __init__(self, path: pathlib.Path) -> None:
        self.path = path
        self.descriptor = -1

    def __enter__(self) -> "_BudgetLock":
        _mkdir(self.path.parent)
        flags = os.O_RDWR | os.O_CREAT | getattr(os, "O_CLOEXEC", 0)
        flags |= getattr(os, "O_NOFOLLOW", 0)
        self.descriptor = os.open(self.path, flags, 0o600)
        fcntl.flock(self.descriptor, fcntl.LOCK_EX)
        return self

    def __exit__(self, *_: object) -> None:
        if self.descriptor >= 0:
            fcntl.flock(self.descriptor, fcntl.LOCK_UN)
            os.close(self.descriptor)
            self.descriptor = -1


class ClassifiedProofBackend:
    """Execute one exact delegation through injected closed plan adapters."""

    def __init__(
        self, state_root: pathlib.Path, adapters: Mapping[str, PlanAdapter] | None = None,
    ) -> None:
        self.state_root = pathlib.Path(state_root)
        self.adapters = MappingProxyType(dict(adapters or PRODUCTION_ADAPTERS))

    def execute(self, authority: Authority, delegation_bytes: bytes) -> ExecutionReceipt:
        delegation = self._readmit_current_authority(authority, delegation_bytes)
        adapter = self.adapters.get(str(delegation["plan_id"]))
        if adapter is None:
            raise UnsupportedAdapter("requested plan has no production adapter")
        descriptor = self._admit_adapter(adapter, delegation)
        preflight_context = PreflightContext(
            delegation=_json_object(delegation),
            plan_descriptor=_json_object(descriptor),
        )
        try:
            adapter.preflight(preflight_context)
        except Exception as exc:
            raise PreflightFailed("closed adapter read-only preflight failed") from exc

        reservation_identity = self._reservation_identity(delegation)
        paths = self._paths(delegation, reservation_identity)
        prior = self._read_transaction_if_present(paths, delegation, descriptor)
        reconciled: Observation | None = None
        readmitted_result: dict[str, Any] | None = None
        prior_revision: int | None = None
        if prior is not None:
            prior_revision = int(prior["revision"])
            if prior["result_file"] is not None:
                readmitted_result = self._readmit_result(adapter, paths, prior)
            elif self._needs_reconciliation(prior):
                reconciled = self._reconcile(adapter, delegation, descriptor, prior)
            elif prior["failure_file"] is not None:
                self._verify_failure_pair(paths, prior)

        # Reject drift during preflight/reconciliation before creating even the
        # budget lock. Re-admit once more under that lock before durable writes.
        self._readmit_current_authority(authority, delegation_bytes)
        with _BudgetLock(paths.lock):
            self._readmit_current_authority(authority, delegation_bytes)
            budget = self._load_or_initialize_budget(paths, delegation, authority)
            transaction = self._read_transaction_if_present(
                paths, delegation, descriptor,
            )
            if transaction is None:
                if reservation_identity in budget["reservations"]:
                    raise DurableStateError("budget reservation has no transaction record")
                if int(budget["consumed_count"]) >= int(
                    budget["batch_budget_maximum"]
                ):
                    raise BudgetExhausted(
                        "stable execution identity has exhausted its budget"
                    )
            elif self._needs_reconciliation(transaction):
                if prior_revision != int(transaction["revision"]):
                    reconciled = self._reconcile(
                        adapter, delegation, descriptor, transaction,
                    )
            if transaction is not None and prior_revision != int(transaction["revision"]):
                readmitted_result = None

            if reservation_identity in budget["reservations"]:
                if transaction is None:
                    raise DurableStateError(
                        "budget reservation has no transaction record"
                    )
                if transaction["state"] == TransactionState.PREFLIGHTED.value:
                    transaction = self._retain_unknown(
                        paths, transaction, budget,
                        effects={"workload_launches": "unknown"},
                        cleanup="UNKNOWN", protected="UNKNOWN",
                    )
                return self._resume(
                    adapter, paths, transaction, budget, reconciled,
                    readmitted_result,
                )

            if (transaction is not None
                    and transaction["state"] != TransactionState.PREFLIGHTED.value):
                raise DurableStateError("unreserved transaction is not PREFLIGHTED")
            if int(budget["consumed_count"]) >= int(budget["batch_budget_maximum"]):
                raise BudgetExhausted("stable execution identity has exhausted its budget")

            # This is the final read-only boundary before transaction retention
            # and reservation publication. A stale authority creates neither.
            self._readmit_current_authority(authority, delegation_bytes)
            if transaction is None:
                transaction = self._new_transaction(
                    delegation, delegation_bytes, descriptor, reservation_identity,
                )
                _mkdir(paths.transaction_dir)
                self._write_transaction(paths, transaction)

            # The budget file is authoritative. If a crash occurs before the
            # following transaction update, reconciliation treats the batch as
            # consumed and never launches it speculatively.
            budget["reservations"].append(reservation_identity)
            budget["consumed_count"] = len(budget["reservations"])
            self._write_budget(paths, budget)
            transaction = self._transition(transaction, TransactionState.RESERVED)
            self._write_transaction(paths, transaction)
            transaction = self._transition(transaction, TransactionState.LAUNCHING)
            self._write_transaction(paths, transaction)
            context = self._reservation_context(
                delegation, descriptor, reservation_identity, transaction,
            )
            try:
                observation = adapter.invoke(context)
                if not isinstance(observation, Observation):
                    raise BackendError("adapter invocation returned no typed observation")
                observation.record()
            except OutcomeUnknown as exc:
                observation = self._reconcile(
                    adapter, delegation, descriptor, transaction,
                )
                if observation is None:
                    transaction = self._retain_unknown(
                        paths, transaction, budget,
                        effects=exc.effects,
                        cleanup=exc.cleanup_disposition,
                        protected=exc.protected_state_disposition,
                    )
                    return self._receipt(transaction, budget)
            except Exception:
                observation = self._reconcile(
                    adapter, delegation, descriptor, transaction,
                )
                if observation is None:
                    transaction = self._retain_unknown(
                        paths, transaction, budget,
                        effects={"workload_launches": "unknown"},
                        cleanup="UNKNOWN", protected="UNKNOWN",
                    )
                    return self._receipt(transaction, budget)
            return self._retain_observation(
                adapter, paths, transaction, budget, observation,
            )

    def render_retained(self, authority: Authority, delegation_bytes: bytes,
                        *, renderer_revision: str) -> ExecutionReceipt:
        """Explicit local reporting recovery; never preflight, reconcile or invoke."""
        delegation = self._readmit_current_authority(authority, delegation_bytes)
        adapter = self.adapters.get(str(delegation["plan_id"]))
        if not isinstance(adapter, AcceptancePlanAdapter) or HEX64.fullmatch(renderer_revision) is None:
            raise BackendError("retained rendering requires acceptance and a renderer revision")
        descriptor = self._admit_adapter(adapter, delegation)
        paths = self._paths(delegation, self._reservation_identity(delegation))
        with _BudgetLock(paths.lock):
            transaction = self._read_transaction_if_present(paths, delegation, descriptor)
            if transaction is None or transaction["result_file"] is None:
                raise DurableStateError("no immutable acceptance result to render")
            budget = self._load_or_initialize_budget(paths, delegation, authority)
            if transaction["reservation_identity"] not in budget["reservations"]:
                raise DurableStateError("acceptance result has no consumed reservation")
            result = self._readmit_result(adapter, paths, transaction)
            if transaction["renderer_completed"]:
                return self._receipt(transaction, budget)
            recovery = paths.transaction_dir / ("renderer-recovery-"+renderer_revision+".json")
            if recovery.exists():
                raise DurableStateError("this renderer recovery revision was already attempted")
            record = {"renderer_revision":renderer_revision,
                      "result_sha256":transaction["result_sha256"], "state":"STARTED",
                      "workload_calls":0}
            _write_object(recovery, record, maximum=STATE_MAX_BYTES)
            context = self._reservation_context(delegation, descriptor,
                transaction["reservation_identity"], transaction)
            try:
                adapter.render_product_evidence(context, _json_object(result))
            except Exception:
                _write_object(recovery, {**record,"state":"FAILED"}, maximum=STATE_MAX_BYTES)
                raise
            _write_object(recovery, {**record,"state":"COMPLETE"}, maximum=STATE_MAX_BYTES)
            transaction = self._transition(self._update(transaction,
                renderer_reservation_consumed=True, renderer_completed=True),
                TransactionState.ACCEPTANCE_EVIDENCE_RENDERED)
            self._write_transaction(paths, transaction)
            return self._receipt(transaction, budget)

    def _readmit_current_authority(
        self, authority: Authority, delegation_bytes: bytes,
    ) -> dict[str, Any]:
        try:
            current = load_authority(authority.path)
            if (current.raw_sha256 != authority.raw_sha256
                    or dict(current.fields) != dict(authority.fields)):
                raise PolicyError("on-disk authority differs from the supplied authority")
            return validate_canonical_delegation(current, delegation_bytes)
        except (PolicyError, OSError) as exc:
            raise BackendError(
                "current on-disk authority or canonical delegation was not admitted"
            ) from exc

    def _admit_adapter(
        self, adapter: PlanAdapter, delegation: Mapping[str, Any],
    ) -> dict[str, Any]:
        if not isinstance(adapter, (DiagnosticPlanAdapter, AcceptancePlanAdapter)):
            raise UnsupportedAdapter("adapter type is outside the closed PX2 set")
        adapter.descriptor.validate()
        expected_class = ExecutionClass(str(delegation["execution_class"]))
        if adapter.descriptor.execution_class is not expected_class:
            raise UnsupportedAdapter("adapter execution class differs from delegation")
        if expected_class is ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE:
            if not isinstance(adapter, DiagnosticPlanAdapter):
                raise UnsupportedAdapter("diagnostic plan was given renderer capability")
            if delegation["acceptance_eligible"] is not False:
                raise BackendError("diagnostic delegation is acceptance-eligible")
        elif not isinstance(adapter, AcceptancePlanAdapter):
            raise UnsupportedAdapter("acceptance plan has no product renderer capability")
        record = adapter.descriptor.record()
        joins = {
            "plan_id": "plan_id",
            "product_contract_identity": "product_contract_identity",
            "product_contract_sha256": "product_contract_sha256",
            "plan_content_sha256": "plan_content_sha256",
        }
        for descriptor_key, delegation_key in joins.items():
            if record[descriptor_key] != delegation[delegation_key]:
                raise BackendError(f"adapter {descriptor_key} differs from delegation")
        return record

    def _reservation_identity(self, delegation: Mapping[str, Any]) -> str:
        value = {
            "schema": RESERVATION_SCHEMA,
            "execution_class": delegation["execution_class"],
            "execution_identity": delegation["execution_identity"],
            "source_commit": delegation["source_commit"],
            "product_contract_identity": delegation["product_contract_identity"],
            "product_contract_sha256": delegation["product_contract_sha256"],
            "plan_id": delegation["plan_id"],
            "plan_content_sha256": delegation["plan_content_sha256"],
        }
        return sha256_bytes(canonical_json(value))

    def _paths(
        self, delegation: Mapping[str, Any], reservation_identity: str,
    ) -> _Paths:
        identity = str(delegation["execution_identity"])
        if HEX64.fullmatch(identity) is None or HEX64.fullmatch(reservation_identity) is None:
            raise BackendError("unsafe durable identity")
        class_directory = {
            ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE.value: "diagnostic",
            ExecutionClass.ACCEPTANCE_CANDIDATE.value: "acceptance",
        }[str(delegation["execution_class"])]
        owner = self.state_root / class_directory / identity
        transaction_dir = owner / "transactions" / reservation_identity
        return _Paths(
            owner=owner,
            lock=owner / "budget.lock",
            budget=owner / "budget.json",
            transaction_dir=transaction_dir,
            transaction=transaction_dir / "transaction.json",
            result=transaction_dir / "classified-result.json",
            failure=transaction_dir / "classified-failure.json",
            failure_sidecar=transaction_dir / "classified-failure.json.sha256",
        )

    def _budget_template(self, delegation: Mapping[str, Any]) -> dict[str, Any]:
        acceptance = delegation["execution_class"] == ExecutionClass.ACCEPTANCE_CANDIDATE.value
        return {
            "schema": BUDGET_SCHEMA,
            "execution_class": delegation["execution_class"],
            "execution_identity": delegation["execution_identity"],
            "product_contract_identity": delegation["product_contract_identity"],
            "product_contract_sha256": delegation["product_contract_sha256"],
            "plan_id": delegation["plan_id"],
            "plan_content_sha256": delegation["plan_content_sha256"],
            "candidate_source_commit": delegation["source_commit"] if acceptance else None,
            "batch_budget_maximum": delegation["batch_budget_maximum"],
            "consumed_count": 0,
            "reservations": [],
        }

    def _load_or_initialize_budget(
        self, paths: _Paths, delegation: Mapping[str, Any], authority: Authority | None = None,
    ) -> dict[str, Any]:
        expected = self._budget_template(delegation)
        if not paths.budget.exists():
            return expected
        budget = _read_object(paths.budget)
        keys = set(expected)
        if set(budget) != keys or budget.get("schema") != BUDGET_SCHEMA:
            raise DurableStateError("budget key roster or schema differs")
        revision_keys = {"plan_content_sha256", "product_contract_sha256"}
        for key in keys - {"consumed_count", "reservations", "batch_budget_maximum"} - revision_keys:
            if budget[key] != expected[key]:
                raise DurableStateError(f"stable budget binding differs: {key}")
        reservations = budget["reservations"]
        if (not isinstance(reservations, list)
                or any(not isinstance(item, str) or HEX64.fullmatch(item) is None
                       for item in reservations)
                or len(set(reservations)) != len(reservations)):
            raise DurableStateError("budget reservations are malformed")
        if budget["consumed_count"] != len(reservations):
            raise DurableStateError("budget consumed count differs from reservations")
        if (not isinstance(budget["consumed_count"], int)
                or isinstance(budget["consumed_count"], bool)):
            raise DurableStateError("budget consumed count is not an integer")
        old_maximum = budget["batch_budget_maximum"]
        maximum = expected["batch_budget_maximum"]
        if type(old_maximum) is not int or old_maximum < 1:
            raise DurableStateError("budget maximum is not a positive integer")
        if len(reservations) > old_maximum:
            raise DurableStateError("budget exceeds its maximum")
        if any(budget[key] != expected[key] for key in revision_keys):
            # A diagnostic repair can bind a new exact artifact/plan under the
            # SAME campaign. Only explicit old-to-current authority admits it;
            # acceptance and historical reservations are never rebound.
            if (authority is None
                    or delegation["execution_class"] != ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE.value
                    or delegation["acceptance_eligible"] is not False
                    or old_maximum != maximum
                    or authority.fields.get("diagnostic_plan_revision_authorized") != "true"
                    or any(authority.fields.get("diagnostic_previous_" + key) != budget[key]
                           for key in revision_keys)):
                raise DurableStateError("stable budget binding differs: unauthorized plan revision")
            current = load_authority(authority.path)
            if current.raw_sha256 != delegation["authority_sha256"]:
                raise DurableStateError("current on-disk authority differs before plan revision")
            updated = {**budget, **{key: expected[key] for key in revision_keys}}
            adjustment = {
                "schema": "classified-proof-diagnostic-plan-adjustment/v1",
                "authority_sha256": delegation["authority_sha256"],
                "before": budget, "after": updated,
            }
            adjustment_path = paths.budget.with_name(
                "plan-adjustment-" + sha256_bytes(canonical_json(
                    {key: [budget[key], updated[key]] for key in sorted(revision_keys)})) + ".json")
            if adjustment_path.exists():
                if _read_object(adjustment_path) != adjustment:
                    raise DurableStateError("diagnostic plan adjustment record differs")
            else:
                _write_object(adjustment_path, adjustment, maximum=STATE_MAX_BYTES)
            self._write_budget(paths, updated)
            budget = updated
        if old_maximum != maximum:
            if (delegation["execution_class"] != ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE.value
                    or delegation["acceptance_eligible"] is not False or maximum < old_maximum):
                raise DurableStateError("budget decrease or acceptance extension forbidden")
            updated = {**budget, "batch_budget_maximum": maximum}
            # Called only under the existing budget lock after fresh authority
            # admission. Record intent before the atomic replacement, so an
            # interrupted replacement is safely repeatable without count loss.
            adjustment = {
                "schema": "classified-proof-budget-adjustment/v1",
                "authority_sha256": delegation["authority_sha256"],
                "before": budget, "after": updated,
            }
            adjustment_path = paths.budget.with_name(
                f"budget-adjustment-{old_maximum}-to-{maximum}.json")
            if adjustment_path.exists():
                if _read_object(adjustment_path) != adjustment:
                    raise DurableStateError("budget adjustment record differs")
            else:
                _write_object(adjustment_path, adjustment, maximum=STATE_MAX_BYTES)
            self._write_budget(paths, updated)
            budget = updated
        return budget

    def _write_budget(self, paths: _Paths, budget: Mapping[str, Any]) -> None:
        _write_object(paths.budget, budget, maximum=STATE_MAX_BYTES)

    def _new_transaction(
        self,
        delegation: Mapping[str, Any],
        delegation_bytes: bytes,
        descriptor: Mapping[str, Any],
        reservation_identity: str,
    ) -> dict[str, Any]:
        return {
            "schema": TRANSACTION_SCHEMA,
            "revision": 1,
            "state": TransactionState.PREFLIGHTED.value,
            "history": [TransactionState.NEW.value, TransactionState.PREFLIGHTED.value],
            "delegation": _json_object(delegation),
            "delegation_sha256": sha256_bytes(delegation_bytes),
            "plan_descriptor": _json_object(descriptor),
            "reservation_identity": reservation_identity,
            "observation": None,
            "result_file": None,
            "result_sha256": None,
            "failure_file": None,
            "failure_sha256": None,
            "renderer_reservation_consumed": False,
            "renderer_completed": False,
        }

    def _read_transaction_if_present(
        self,
        paths: _Paths,
        delegation: Mapping[str, Any],
        descriptor: Mapping[str, Any],
    ) -> dict[str, Any] | None:
        if not paths.transaction.exists():
            return None
        transaction = _read_object(paths.transaction)
        self._validate_transaction(
            transaction, delegation, descriptor, paths.transaction_dir.name,
        )
        return transaction

    def _validate_transaction(
        self,
        transaction: Mapping[str, Any],
        delegation: Mapping[str, Any],
        descriptor: Mapping[str, Any],
        reservation_identity: str,
    ) -> None:
        keys = {
            "schema", "revision", "state", "history", "delegation",
            "delegation_sha256", "plan_descriptor", "reservation_identity",
            "observation", "result_file", "result_sha256", "failure_file",
            "failure_sha256", "renderer_reservation_consumed", "renderer_completed",
        }
        if set(transaction) != keys or transaction.get("schema") != TRANSACTION_SCHEMA:
            raise DurableStateError("transaction key roster or schema differs")
        original = transaction["delegation"]
        if not isinstance(original, dict) or set(original) != set(delegation):
            raise DurableStateError("transaction delegation differs")
        if original != dict(delegation):
            mutable = {"authority_sha256", "batch_budget_maximum"}
            if (delegation["execution_class"] != ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE.value
                    or any(original[k] != delegation[k] for k in set(delegation) - mutable)
                    or type(original["batch_budget_maximum"]) is not int
                    or not 0 < original["batch_budget_maximum"] <= delegation["batch_budget_maximum"]
                    or not isinstance(original["authority_sha256"], str)
                    or HEX64.fullmatch(original["authority_sha256"]) is None):
                raise DurableStateError("transaction delegation differs")
        if transaction["delegation_sha256"] != sha256_bytes(canonical_json(original)):
            raise DurableStateError("transaction delegation digest differs")
        if transaction["plan_descriptor"] != dict(descriptor):
            raise DurableStateError("transaction plan descriptor differs")
        if transaction["reservation_identity"] != reservation_identity:
            raise DurableStateError("transaction reservation identity differs")
        if (not isinstance(transaction["revision"], int)
                or isinstance(transaction["revision"], bool)
                or transaction["revision"] < 1):
            raise DurableStateError("transaction revision is malformed")
        try:
            TransactionState(str(transaction["state"]))
        except ValueError as exc:
            raise DurableStateError("transaction state is unknown") from exc
        history = transaction["history"]
        if (not isinstance(history, list) or len(history) < 2
                or history[0] != TransactionState.NEW.value
                or history[-1] != transaction["state"]):
            raise DurableStateError("transaction history is malformed")
        for before, after in zip(history, history[1:]):
            try:
                before_state = TransactionState(before)
                after_state = TransactionState(after)
            except ValueError as exc:
                raise DurableStateError("transaction history has unknown state") from exc
            if after_state not in TRANSITIONS[before_state]:
                raise DurableStateError("transaction history is not monotonic")
        for key in ("renderer_reservation_consumed", "renderer_completed"):
            if not isinstance(transaction[key], bool):
                raise DurableStateError(f"transaction {key} is malformed")
        if transaction["renderer_completed"] and not transaction["renderer_reservation_consumed"]:
            raise DurableStateError("renderer completed without durable reservation")
        if (transaction["delegation"]["execution_class"]
                == ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE.value
                and (transaction["renderer_reservation_consumed"]
                     or transaction["renderer_completed"])):
            raise DurableStateError("diagnostic transaction acquired renderer state")
        if transaction["observation"] is not None:
            if not isinstance(transaction["observation"], Mapping):
                raise DurableStateError("transaction observation is malformed")
            Observation.from_record(transaction["observation"])
        for file_key, digest_key, expected_name in (
            ("result_file", "result_sha256", "classified-result.json"),
            ("failure_file", "failure_sha256", "classified-failure.json"),
        ):
            file_value = transaction[file_key]
            digest_value = transaction[digest_key]
            if (file_value is None) != (digest_value is None):
                raise DurableStateError(f"transaction {file_key} binding is partial")
            if file_value is not None:
                if file_value != expected_name or HEX64.fullmatch(str(digest_value)) is None:
                    raise DurableStateError(f"transaction {file_key} binding differs")

    def _write_transaction(self, paths: _Paths, transaction: Mapping[str, Any]) -> None:
        _write_object(paths.transaction, transaction, maximum=STATE_MAX_BYTES)

    def _transition(
        self, transaction: Mapping[str, Any], state: TransactionState,
        **updates: Any,
    ) -> dict[str, Any]:
        current = TransactionState(str(transaction["state"]))
        if state not in TRANSITIONS[current]:
            raise DurableStateError(f"illegal transaction transition {current.value}->{state.value}")
        value = dict(transaction)
        value.update(updates)
        value["revision"] = int(value["revision"]) + 1
        value["state"] = state.value
        value["history"] = [*transaction["history"], state.value]
        return value

    def _update(self, transaction: Mapping[str, Any], **updates: Any) -> dict[str, Any]:
        value = dict(transaction)
        value.update(updates)
        value["revision"] = int(value["revision"]) + 1
        return value

    def _needs_reconciliation(self, transaction: Mapping[str, Any]) -> bool:
        return transaction["state"] in {
            TransactionState.RESERVED.value,
            TransactionState.LAUNCHING.value,
            TransactionState.OUTCOME_UNKNOWN.value,
        }

    def _reservation_context(
        self,
        delegation: Mapping[str, Any],
        descriptor: Mapping[str, Any],
        reservation_identity: str,
        prior: Mapping[str, Any] | None,
    ) -> ReservationContext:
        return ReservationContext(
            delegation=_json_object(delegation),
            plan_descriptor=_json_object(descriptor),
            reservation_identity=reservation_identity,
            prior_transaction=_json_object(prior) if prior is not None else None,
        )

    def _reconcile(
        self,
        adapter: PlanAdapter,
        delegation: Mapping[str, Any],
        descriptor: Mapping[str, Any],
        transaction: Mapping[str, Any],
    ) -> Observation | None:
        context = self._reservation_context(
            transaction["delegation"], descriptor, str(transaction["reservation_identity"]), transaction,
        )
        try:
            observation = adapter.reconcile(context)
        except Exception:
            return None
        if observation is not None and not isinstance(observation, Observation):
            raise BackendError("adapter reconciliation returned an untyped observation")
        if observation is not None:
            try:
                observation.record()
            except Exception:
                return None
        return observation

    def _resume(
        self,
        adapter: PlanAdapter,
        paths: _Paths,
        transaction: dict[str, Any],
        budget: dict[str, Any],
        reconciled: Observation | None,
        readmitted_result: Mapping[str, Any] | None,
    ) -> ExecutionReceipt:
        state = TransactionState(str(transaction["state"]))
        if state in {
            TransactionState.RESERVED,
            TransactionState.LAUNCHING,
            TransactionState.OUTCOME_UNKNOWN,
        }:
            if reconciled is None:
                reconciled = self._reconcile(
                    adapter,
                    transaction["delegation"],
                    transaction["plan_descriptor"],
                    transaction,
                )
            if reconciled is None:
                if state is not TransactionState.OUTCOME_UNKNOWN:
                    transaction = self._retain_unknown(
                        paths, transaction, budget,
                        effects={"workload_launches": "unknown"},
                        cleanup="UNKNOWN", protected="UNKNOWN",
                    )
                else:
                    self._verify_failure_pair(paths, transaction)
                return self._receipt(transaction, budget)
            return self._retain_observation(
                adapter, paths, transaction, budget, reconciled,
            )
        if state is TransactionState.OBSERVED:
            if not isinstance(transaction["observation"], Mapping):
                raise DurableStateError("OBSERVED transaction has no observation")
            return self._process_observation(
                adapter,
                paths,
                transaction,
                budget,
                Observation.from_record(transaction["observation"]),
            )
        if transaction["result_file"] is not None:
            return self._resume_result(
                adapter, paths, transaction, budget, readmitted_result,
            )
        if transaction["failure_file"] is not None:
            self._verify_failure_pair(paths, transaction)
            return self._receipt(transaction, budget)
        raise DurableStateError("consumed transaction has no reconcilable disposition")

    def _retain_observation(
        self,
        adapter: PlanAdapter,
        paths: _Paths,
        transaction: dict[str, Any],
        budget: dict[str, Any],
        observation: Observation,
    ) -> ExecutionReceipt:
        if TransactionState(str(transaction["state"])) not in {
            TransactionState.LAUNCHING, TransactionState.OUTCOME_UNKNOWN,
            TransactionState.RESERVED,
        }:
            raise DurableStateError("observation arrived outside a reconcilable state")
        transaction = self._transition(
            transaction, TransactionState.OBSERVED, observation=observation.record(),
        )
        self._write_transaction(paths, transaction)
        return self._process_observation(
            adapter, paths, transaction, budget, observation,
        )

    def _process_observation(
        self,
        adapter: PlanAdapter,
        paths: _Paths,
        transaction: dict[str, Any],
        budget: dict[str, Any],
        observation: Observation,
    ) -> ExecutionReceipt:
        if observation.kind is not ObservationKind.SUCCESS:
            transaction = self._retain_failure(
                paths, transaction, budget,
                classification=observation.classification,
                effects=observation.effects,
                cleanup=observation.cleanup_disposition,
                protected=observation.protected_state_disposition,
            )
            transaction = self._transition(transaction, TransactionState.FAILURE_RETAINED)
            self._write_transaction(paths, transaction)
            transaction = self._transition(transaction, TransactionState.CLOSED)
            self._write_transaction(paths, transaction)
            return self._receipt(transaction, budget)

        context = self._reservation_context(
            transaction["delegation"], transaction["plan_descriptor"],
            str(transaction["reservation_identity"]), transaction,
        )
        try:
            admitted = adapter.admit(context, observation)
            admitted = _bounded_object(admitted, RESULT_MAX_BYTES, "admitted result")
        except Exception:
            transaction = self._retain_failure(
                paths, transaction, budget,
                classification="RESULT_ADMISSION_FAILED",
                effects=observation.effects,
                cleanup=observation.cleanup_disposition,
                protected=observation.protected_state_disposition,
            )
            transaction = self._transition(transaction, TransactionState.FAILURE_RETAINED)
            self._write_transaction(paths, transaction)
            transaction = self._transition(transaction, TransactionState.CLOSED)
            self._write_transaction(paths, transaction)
            return self._receipt(transaction, budget)

        is_acceptance = isinstance(adapter, AcceptancePlanAdapter)
        if is_acceptance and (
            transaction["delegation"]["acceptance_eligible"] is not True
            or observation.cleanup_disposition != "COMPLETE"
            or observation.protected_state_disposition != "UNCHANGED"
            or "unknown" in observation.effects.values()
        ):
            transaction = self._retain_failure(
                paths, transaction, budget,
                classification="ACCEPTANCE_DISPOSITION_INADMISSIBLE",
                effects=observation.effects,
                cleanup=observation.cleanup_disposition,
                protected=observation.protected_state_disposition,
            )
            transaction = self._transition(transaction, TransactionState.FAILURE_RETAINED)
            self._write_transaction(paths, transaction)
            transaction = self._transition(transaction, TransactionState.CLOSED)
            self._write_transaction(paths, transaction)
            return self._receipt(transaction, budget)

        result = self._result_record(transaction, observation, admitted)
        result_sha256 = _write_object(paths.result, result, maximum=RESULT_ENVELOPE_MAX_BYTES)
        transaction = self._transition(
            transaction,
            TransactionState.RESULT_RETAINED,
            result_file=paths.result.name,
            result_sha256=result_sha256,
        )
        self._write_transaction(paths, transaction)
        transaction = self._transition(transaction, TransactionState.CLOSED)
        self._write_transaction(paths, transaction)
        if is_acceptance:
            return self._render_acceptance(adapter, paths, transaction, budget, result)
        return self._receipt(transaction, budget)

    def _result_record(
        self,
        transaction: Mapping[str, Any],
        observation: Observation,
        admitted: Mapping[str, Any],
    ) -> dict[str, Any]:
        delegation = transaction["delegation"]
        return {
            "schema": RESULT_SCHEMA,
            "authority_sha256": delegation["authority_sha256"],
            "execution_class": delegation["execution_class"],
            "execution_identity": delegation["execution_identity"],
            "acceptance_eligible": delegation["acceptance_eligible"],
            "reservation_identity": transaction["reservation_identity"],
            "source_commit": delegation["source_commit"],
            "product_contract_identity": delegation["product_contract_identity"],
            "product_contract_sha256": delegation["product_contract_sha256"],
            "plan_id": delegation["plan_id"],
            "plan_content_sha256": delegation["plan_content_sha256"],
            "observation": observation.record(),
            "admitted_result": dict(admitted),
        }

    def _resume_result(
        self,
        adapter: PlanAdapter,
        paths: _Paths,
        transaction: dict[str, Any],
        budget: dict[str, Any],
        readmitted_result: Mapping[str, Any] | None = None,
    ) -> ExecutionReceipt:
        result = (
            _json_object(readmitted_result)
            if readmitted_result is not None
            else self._readmit_result(adapter, paths, transaction)
        )
        state = TransactionState(str(transaction["state"]))
        if state is TransactionState.RESULT_RETAINED:
            transaction = self._transition(transaction, TransactionState.CLOSED)
            self._write_transaction(paths, transaction)
        if isinstance(adapter, AcceptancePlanAdapter):
            return self._render_acceptance(adapter, paths, transaction, budget, result)
        return self._receipt(transaction, budget)

    def _readmit_result(
        self,
        adapter: PlanAdapter,
        paths: _Paths,
        transaction: Mapping[str, Any],
    ) -> dict[str, Any]:
        result = _read_object(paths.result, maximum=RESULT_ENVELOPE_MAX_BYTES)
        raw = canonical_json(result)
        if sha256_bytes(raw) != transaction["result_sha256"]:
            raise DurableStateError("retained result digest differs")
        expected_keys = {
            "schema", "authority_sha256", "execution_class", "execution_identity",
            "acceptance_eligible", "reservation_identity", "source_commit",
            "product_contract_identity", "product_contract_sha256", "plan_id",
            "plan_content_sha256", "observation", "admitted_result",
        }
        if set(result) != expected_keys or result.get("schema") != RESULT_SCHEMA:
            raise DurableStateError("retained result key roster or schema differs")
        delegation = transaction["delegation"]
        for key in (
            "authority_sha256", "execution_class", "execution_identity",
            "acceptance_eligible", "source_commit", "product_contract_identity",
            "product_contract_sha256", "plan_id", "plan_content_sha256",
        ):
            if result[key] != delegation[key]:
                raise DurableStateError(f"retained result binding differs: {key}")
        if result["reservation_identity"] != transaction["reservation_identity"]:
            raise DurableStateError("retained result reservation differs")
        if transaction["failure_file"] is not None:
            self._verify_failure_pair(paths, transaction)
        observation = Observation.from_record(result["observation"])
        if observation.kind is not ObservationKind.SUCCESS:
            raise DurableStateError("retained result is not a successful observation")
        if isinstance(adapter, AcceptancePlanAdapter) and (
            result["acceptance_eligible"] is not True
            or observation.cleanup_disposition != "COMPLETE"
            or observation.protected_state_disposition != "UNCHANGED"
            or "unknown" in observation.effects.values()
        ):
            raise DurableStateError("retained acceptance disposition is inadmissible")
        context = self._reservation_context(
            delegation, transaction["plan_descriptor"],
            str(transaction["reservation_identity"]), transaction,
        )
        try:
            admitted = _bounded_object(
                adapter.admit(context, observation), RESULT_MAX_BYTES, "admitted result",
            )
        except Exception as exc:
            raise DurableStateError("retained result did not re-admit") from exc
        if admitted != result["admitted_result"]:
            raise DurableStateError("retained admitted result differs")
        return result

    def _render_acceptance(
        self,
        adapter: AcceptancePlanAdapter,
        paths: _Paths,
        transaction: dict[str, Any],
        budget: dict[str, Any],
        result: Mapping[str, Any],
    ) -> ExecutionReceipt:
        if transaction["renderer_completed"]:
            return self._receipt(transaction, budget)
        if transaction["renderer_reservation_consumed"]:
            if transaction["failure_file"] is None:
                transaction = self._retain_failure(
                    paths, transaction, budget,
                    classification="EVIDENCE_RENDER_OUTCOME_UNKNOWN",
                    effects=result["observation"]["effects"],
                    cleanup=result["observation"]["cleanup_disposition"],
                    protected=result["observation"]["protected_state_disposition"],
                )
                self._write_transaction(paths, transaction)
            return self._receipt(transaction, budget)
        # Claim the one renderer call before invoking it. A crash can lose the
        # render but cannot cause a duplicate product-evidence invocation.
        transaction = self._update(
            transaction, renderer_reservation_consumed=True,
        )
        self._write_transaction(paths, transaction)
        context = self._reservation_context(
            transaction["delegation"], transaction["plan_descriptor"],
            str(transaction["reservation_identity"]), transaction,
        )
        try:
            adapter.render_product_evidence(context, _json_object(result))
        except Exception:
            transaction = self._retain_failure(
                paths, transaction, budget,
                classification="EVIDENCE_RENDER_FAILED",
                effects=result["observation"]["effects"],
                cleanup=result["observation"]["cleanup_disposition"],
                protected=result["observation"]["protected_state_disposition"],
            )
            self._write_transaction(paths, transaction)
            return self._receipt(transaction, budget)
        transaction = self._transition(
            self._update(transaction, renderer_completed=True),
            TransactionState.ACCEPTANCE_EVIDENCE_RENDERED,
        )
        self._write_transaction(paths, transaction)
        return self._receipt(transaction, budget)

    def _retain_unknown(
        self,
        paths: _Paths,
        transaction: dict[str, Any],
        budget: Mapping[str, Any],
        *,
        effects: Mapping[str, int | str],
        cleanup: str,
        protected: str,
    ) -> dict[str, Any]:
        transaction = self._retain_failure(
            paths, transaction, budget,
            classification="OUTCOME_UNKNOWN",
            effects=effects,
            cleanup=cleanup,
            protected=protected,
        )
        if transaction["state"] != TransactionState.OUTCOME_UNKNOWN.value:
            transaction = self._transition(transaction, TransactionState.OUTCOME_UNKNOWN)
        self._write_transaction(paths, transaction)
        return transaction

    def _retain_failure(
        self,
        paths: _Paths,
        transaction: dict[str, Any],
        budget: Mapping[str, Any],
        *,
        classification: str,
        effects: Mapping[str, int | str],
        cleanup: str,
        protected: str,
    ) -> dict[str, Any]:
        if transaction["failure_file"] is not None:
            self._verify_failure_pair(paths, transaction)
            return transaction
        if CLASSIFICATION.fullmatch(classification) is None:
            raise BackendError("failure classification is outside the closed syntax")
        validated_effects = _validated_effects(effects)
        if cleanup not in {"COMPLETE", "INCOMPLETE", "UNKNOWN"}:
            raise BackendError("failure cleanup disposition is outside the closed set")
        if protected not in {"UNCHANGED", "RESTORED", "CHANGED", "UNKNOWN"}:
            raise BackendError("failure protected-state disposition is outside the closed set")
        delegation = transaction["delegation"]
        remaining = int(budget["batch_budget_maximum"]) - int(budget["consumed_count"])
        failure = {
            "schema": FAILURE_SCHEMA,
            "authority_sha256": delegation["authority_sha256"],
            "execution_class": delegation["execution_class"],
            "execution_identity": delegation["execution_identity"],
            "acceptance_eligible": delegation["acceptance_eligible"],
            "reservation_identity": transaction["reservation_identity"],
            "source_commit": delegation["source_commit"],
            "product_contract_identity": delegation["product_contract_identity"],
            "product_contract_sha256": delegation["product_contract_sha256"],
            "plan_id": delegation["plan_id"],
            "plan_content_sha256": delegation["plan_content_sha256"],
            "primary_classification": classification,
            "cleanup_disposition": cleanup,
            "protected_state_disposition": protected,
            "effects": validated_effects,
            "remaining_budget": remaining,
            "next_action": (
                "reconcile this retained reservation; never relaunch it"
                if classification == "OUTCOME_UNKNOWN"
                else "new execution requires a distinct exact authority and available budget"
            ),
        }
        raw = canonical_json(failure)
        if len(raw) > FAILURE_MAX_BYTES:
            raise DurableStateError("failure closure exceeds 65536 bytes")
        digest = sha256_bytes(raw)
        _atomic_write(paths.failure, raw)
        sidecar = f"{digest}  {paths.failure.name}\n".encode("ascii")
        _atomic_write(paths.failure_sidecar, sidecar)
        return self._update(
            transaction,
            failure_file=paths.failure.name,
            failure_sha256=digest,
        )

    def _verify_failure_pair(
        self, paths: _Paths, transaction: Mapping[str, Any],
    ) -> dict[str, Any]:
        failure = _read_object(paths.failure, maximum=FAILURE_MAX_BYTES)
        raw = canonical_json(failure)
        digest = sha256_bytes(raw)
        if digest != transaction["failure_sha256"]:
            raise DurableStateError("failure diagnostic digest differs")
        if not paths.failure_sidecar.is_file() or paths.failure_sidecar.is_symlink():
            raise DurableStateError("failure sidecar is absent or unsafe")
        expected = f"{digest}  {paths.failure.name}\n".encode("ascii")
        if paths.failure_sidecar.read_bytes() != expected:
            raise DurableStateError("failure sidecar differs")
        keys = {
            "schema", "authority_sha256", "execution_class", "execution_identity",
            "acceptance_eligible", "reservation_identity", "source_commit",
            "product_contract_identity", "product_contract_sha256", "plan_id",
            "plan_content_sha256", "primary_classification",
            "cleanup_disposition", "protected_state_disposition", "effects",
            "remaining_budget", "next_action",
        }
        if set(failure) != keys or failure.get("schema") != FAILURE_SCHEMA:
            raise DurableStateError("failure diagnostic key roster or schema differs")
        delegation = transaction["delegation"]
        for key in (
            "authority_sha256", "execution_class", "execution_identity",
            "acceptance_eligible", "source_commit", "product_contract_identity",
            "product_contract_sha256", "plan_id", "plan_content_sha256",
        ):
            if failure[key] != delegation[key]:
                raise DurableStateError(f"failure diagnostic binding differs: {key}")
        if failure["reservation_identity"] != transaction["reservation_identity"]:
            raise DurableStateError("failure diagnostic reservation differs")
        if CLASSIFICATION.fullmatch(str(failure["primary_classification"])) is None:
            raise DurableStateError("failure diagnostic classification is malformed")
        _validated_effects(failure["effects"])
        if failure["cleanup_disposition"] not in {"COMPLETE", "INCOMPLETE", "UNKNOWN"}:
            raise DurableStateError("failure diagnostic cleanup disposition differs")
        if failure["protected_state_disposition"] not in {
            "UNCHANGED", "RESTORED", "CHANGED", "UNKNOWN",
        }:
            raise DurableStateError("failure diagnostic protected-state disposition differs")
        if (not isinstance(failure["remaining_budget"], int)
                or isinstance(failure["remaining_budget"], bool)
                or failure["remaining_budget"] < 0):
            raise DurableStateError("failure diagnostic remaining budget is malformed")
        expected_action = (
            "reconcile this retained reservation; never relaunch it"
            if failure["primary_classification"] == "OUTCOME_UNKNOWN"
            else "new execution requires a distinct exact authority and available budget"
        )
        if failure["next_action"] != expected_action:
            raise DurableStateError("failure diagnostic next action differs")
        return failure

    def _receipt(
        self, transaction: Mapping[str, Any], budget: Mapping[str, Any],
    ) -> ExecutionReceipt:
        return ExecutionReceipt(
            state=str(transaction["state"]),
            reservation_identity=str(transaction["reservation_identity"]),
            execution_class=str(transaction["delegation"]["execution_class"]),
            budget_consumed=int(budget["consumed_count"]),
            budget_remaining=(
                int(budget["batch_budget_maximum"]) - int(budget["consumed_count"])
            ),
            result_sha256=transaction["result_sha256"],
            failure_sha256=transaction["failure_sha256"],
            renderer_completed=bool(transaction["renderer_completed"]),
        )


def _json_object(value: Mapping[str, Any] | None) -> dict[str, Any]:
    if value is None:
        raise BackendError("expected one canonical object")
    raw = canonical_json(value)
    parsed = parse_canonical_json(raw, maximum=STATE_MAX_BYTES)
    if not isinstance(parsed, dict):
        raise BackendError("expected one canonical object")
    return parsed


def _bounded_object(value: Any, maximum: int, label: str) -> dict[str, Any]:
    if not isinstance(value, Mapping):
        raise BackendError(f"{label} must be one object")
    raw = canonical_json(value)
    if len(raw) > maximum:
        raise BackendError(f"{label} exceeds its bound")
    parsed = parse_canonical_json(raw, maximum=maximum)
    if not isinstance(parsed, dict):
        raise BackendError(f"{label} must be one object")
    return parsed

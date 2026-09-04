#!/usr/bin/env python3
"""Closed execution-class policy for Linux VST Bridge proof tooling.

The policy admits only the finite authority postures below and emits one exact
canonical delegation. It is an operability guard around the proof harness,
not a security sandbox and not product authority.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
import hashlib
import json
import pathlib
import re
from typing import Any, Mapping


AUTHORITY_SCHEMA = "linux-vst-bridge-proof-execution-authority/v1"
DELEGATION_SCHEMA = "linux-vst-bridge-proof-execution-delegation/v1"
HEX40 = re.compile(r"[0-9a-f]{40}")
HEX64 = re.compile(r"[0-9a-f]{64}")
IDENTIFIER = re.compile(r"[a-z0-9][a-z0-9._-]{0,127}")
PLAN_ID = IDENTIFIER


class PolicyError(RuntimeError):
    """A closed proof-execution policy refusal."""


class ExecutionClass(str, Enum):
    READ_ONLY_RECONCILIATION = "READ_ONLY_RECONCILIATION"
    DIAGNOSTIC_NON_AUTHORITATIVE = "DIAGNOSTIC_NON_AUTHORITATIVE"
    ACCEPTANCE_CANDIDATE = "ACCEPTANCE_CANDIDATE"


# Adding a posture is an authority change, not an implementation convenience.
LEGAL_AUTHORITY_POSTURES = frozenset({
    (
        "no_active_slice",
        "no_active_slice",
        "PROOF_HARNESS_MAINTENANCE",
        False,
        False,
        "none",
        True,
        False,
    ),
    (
        "no_active_slice",
        "no_active_slice",
        "PROOF_HARNESS_MAINTENANCE",
        False,
        False,
        "none",
        True,
        True,
    ),
    (
        "active_proof_harness_maintenance",
        "proof_harness_maintenance",
        "PROOF_HARNESS_MAINTENANCE",
        False,
        False,
        "none",
        False,
        False,
    ),
    (
        "active_proof_harness_maintenance",
        "proof_harness_maintenance",
        "PROOF_HARNESS_MAINTENANCE",
        False,
        False,
        "none",
        True,
        False,
    ),
    (
        "active_diagnostic_campaign",
        "proof_harness_maintenance",
        "PROOF_HARNESS_MAINTENANCE",
        False,
        True,
        ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE.value,
        True,
        True,
    ),
    (
        "active_acceptance_candidate",
        "implementation",
        "PRODUCT_CONTRACT_CHANGE",
        True,
        True,
        ExecutionClass.ACCEPTANCE_CANDIDATE.value,
        True,
        True,
    ),
})


@dataclass(frozen=True)
class Authority:
    fields: Mapping[str, str]
    raw_sha256: str
    path: pathlib.Path

    def boolean(self, key: str) -> bool:
        value = self.fields.get(key)
        if value not in {"true", "false"}:
            raise PolicyError(f"authority field {key!r} must be true or false")
        return value == "true"

    def integer(self, key: str) -> int:
        value = self.fields.get(key, "")
        if not re.fullmatch(r"0|[1-9][0-9]*", value):
            raise PolicyError(f"authority field {key!r} must be a nonnegative integer")
        return int(value)


@dataclass(frozen=True)
class LiveRequest:
    execution_class: ExecutionClass
    source_commit: str
    plan_id: str
    execution_identity: str

    def validate_shape(self) -> None:
        if self.execution_class is ExecutionClass.READ_ONLY_RECONCILIATION:
            raise PolicyError("read-only reconciliation is not a workload request")
        if HEX40.fullmatch(self.source_commit) is None:
            raise PolicyError("source commit must be lowercase 40-hex")
        if PLAN_ID.fullmatch(self.plan_id) is None:
            raise PolicyError("plan ID is outside the closed syntax")
        if HEX64.fullmatch(self.execution_identity) is None:
            raise PolicyError("execution identity must be lowercase 64-hex")


def canonical_json(value: Any) -> bytes:
    return (json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=True,
        allow_nan=False,
    )
            + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, item in pairs:
        if key in value:
            raise PolicyError(f"canonical JSON contains duplicate key: {key}")
        value[key] = item
    return value


def parse_canonical_json(raw: bytes, *, maximum: int = 64 * 1024) -> Any:
    if not isinstance(raw, bytes) or not raw or len(raw) > maximum:
        raise PolicyError("canonical JSON bytes are absent or outside the bound")
    try:
        value = json.loads(
            raw.decode("utf-8", "strict"), object_pairs_hook=_reject_duplicate_pairs,
        )
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise PolicyError("canonical JSON is malformed") from exc
    if canonical_json(value) != raw:
        raise PolicyError("JSON bytes are not canonical")
    return value


def parse_authority_document(raw: bytes, path: pathlib.Path) -> Authority:
    try:
        text = raw.decode("utf-8", "strict")
    except UnicodeDecodeError as exc:
        raise PolicyError("CURRENT_SLICE.md is not UTF-8") from exc
    if "\x00" in text:
        raise PolicyError("CURRENT_SLICE.md contains NUL")
    marker = "## Authority"
    marker_at = text.find(marker)
    if marker_at < 0:
        raise PolicyError("CURRENT_SLICE.md has no Authority section")
    fence_start = re.search(r"^```(?:yaml|text)\s*$", text[marker_at:], re.MULTILINE)
    if fence_start is None:
        raise PolicyError("CURRENT_SLICE.md Authority section has no fenced mapping")
    body_start = marker_at + fence_start.end()
    fence_end = re.search(r"^```\s*$", text[body_start:], re.MULTILINE)
    if fence_end is None:
        raise PolicyError("CURRENT_SLICE.md Authority mapping is unterminated")
    body = text[body_start:body_start + fence_end.start()]
    fields: dict[str, str] = {}
    for line_number, line in enumerate(body.splitlines(), 1):
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        if line[:1].isspace():
            continue
        match = re.fullmatch(r"([A-Za-z][A-Za-z0-9_]*):[ \t]*(.*)", line)
        if match is None:
            raise PolicyError(f"malformed authority line {line_number}")
        key, value = match.group(1), match.group(2).strip()
        if key in fields:
            raise PolicyError(f"duplicate authority key: {key}")
        fields[key] = value
    required = {
        "status", "authority_phase", "change_class",
        "product_implementation_authorized", "live_execution_authorized",
        "permitted_execution_class", "classified_backend_core_ready",
        "classified_backend_ready",
    }
    missing = sorted(required - set(fields))
    if missing:
        raise PolicyError(f"authority is missing keys: {', '.join(missing)}")
    return Authority(fields=fields, raw_sha256=sha256_bytes(raw), path=path)


def load_authority(path: pathlib.Path) -> Authority:
    if not path.is_file() or path.is_symlink():
        raise PolicyError("CURRENT_SLICE.md is absent or unsafe")
    if path.stat().st_size > 256 * 1024:
        raise PolicyError("CURRENT_SLICE.md exceeds policy bound")
    return parse_authority_document(path.read_bytes(), path)


def authority_status(authority: Authority) -> dict[str, Any]:
    permitted = authority.fields["permitted_execution_class"]
    closed = {"none", *(member.value for member in ExecutionClass)}
    if permitted not in closed:
        raise PolicyError("permitted_execution_class is outside the closed set")
    product_authorized = authority.boolean("product_implementation_authorized")
    live_authorized = authority.boolean("live_execution_authorized")
    core_ready = authority.boolean("classified_backend_core_ready")
    backend_ready = authority.boolean("classified_backend_ready")
    posture = (
        authority.fields["status"],
        authority.fields["authority_phase"],
        authority.fields["change_class"],
        product_authorized,
        live_authorized,
        permitted,
        core_ready,
        backend_ready,
    )
    if posture not in LEGAL_AUTHORITY_POSTURES:
        raise PolicyError("authority posture is outside the closed PX2 table")
    return {
        "schema": AUTHORITY_SCHEMA,
        "authority_sha256": authority.raw_sha256,
        "status": authority.fields["status"],
        "authority_phase": authority.fields["authority_phase"],
        "change_class": authority.fields["change_class"],
        "product_implementation_authorized": product_authorized,
        "live_execution_authorized": live_authorized,
        "permitted_execution_class": permitted,
        "classified_backend_core_ready": core_ready,
        "classified_backend_ready": backend_ready,
    }


def _required_live_field(authority: Authority, key: str, pattern: re.Pattern[str]) -> str:
    value = authority.fields.get(key, "")
    if pattern.fullmatch(value) is None:
        raise PolicyError(f"authority field {key!r} has invalid shape")
    return value


def authorize_live_request(authority: Authority, request: LiveRequest) -> dict[str, Any]:
    request.validate_shape()
    status = authority_status(authority)
    if status["live_execution_authorized"] is not True:
        raise PolicyError("LIVE_EXECUTION_FORBIDDEN: current authority disables live execution")
    if status["permitted_execution_class"] != request.execution_class.value:
        raise PolicyError("LIVE_EXECUTION_FORBIDDEN: execution class differs from authority")
    if authority.fields.get("authorized_source_commit") != request.source_commit:
        raise PolicyError("LIVE_EXECUTION_FORBIDDEN: source commit differs from authority")
    if authority.fields.get("authorized_plan_id") != request.plan_id:
        raise PolicyError("LIVE_EXECUTION_FORBIDDEN: closed plan differs from authority")
    if status["classified_backend_core_ready"] is not True:
        raise PolicyError("CLASSIFIED_BACKEND_REQUIRED: transaction core is not ready")
    if status["classified_backend_ready"] is not True:
        raise PolicyError("CLASSIFIED_BACKEND_REQUIRED: live backend is not class-aware")

    product_contract_identity = _required_live_field(
        authority, "authorized_product_contract_identity", IDENTIFIER,
    )
    product_contract_sha256 = _required_live_field(
        authority, "authorized_product_contract_sha256", HEX64,
    )
    plan_content_sha256 = _required_live_field(
        authority, "authorized_plan_content_sha256", HEX64,
    )

    if request.execution_class is ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE:
        identity_key = "diagnostic_campaign_identity"
        budget_key = "diagnostic_batch_budget"
        maximum = authority.integer(budget_key)
        if maximum < 1:
            raise PolicyError("diagnostic batch budget must be an explicit positive finite integer")
        acceptance_eligible = False
    else:
        identity_key = "acceptance_candidate_identity"
        budget_key = "acceptance_batch_budget"
        maximum = authority.integer(budget_key)
        if maximum != 1:
            raise PolicyError("acceptance batch budget must be exactly one")
        acceptance_eligible = True

    if authority.fields.get(identity_key) != request.execution_identity:
        raise PolicyError("LIVE_EXECUTION_FORBIDDEN: execution identity differs from authority")

    return {
        "schema": DELEGATION_SCHEMA,
        "authority_sha256": authority.raw_sha256,
        "execution_class": request.execution_class.value,
        "execution_identity": request.execution_identity,
        "source_commit": request.source_commit,
        "plan_id": request.plan_id,
        "product_contract_identity": product_contract_identity,
        "product_contract_sha256": product_contract_sha256,
        "plan_content_sha256": plan_content_sha256,
        "acceptance_eligible": acceptance_eligible,
        "batch_budget_maximum": maximum,
    }


def validate_delegation(
    authority: Authority, value: Mapping[str, Any],
) -> dict[str, Any]:
    keys = {
        "schema", "authority_sha256", "execution_class", "execution_identity",
        "source_commit", "plan_id", "product_contract_identity",
        "product_contract_sha256", "plan_content_sha256",
        "acceptance_eligible", "batch_budget_maximum",
    }
    if not isinstance(value, Mapping) or set(value) != keys:
        raise PolicyError("delegation key roster differs")
    if value.get("schema") != DELEGATION_SCHEMA:
        raise PolicyError("delegation schema differs")
    try:
        execution_class = ExecutionClass(str(value["execution_class"]))
    except ValueError as exc:
        raise PolicyError("delegation execution class differs") from exc
    request = LiveRequest(
        execution_class=execution_class,
        source_commit=str(value["source_commit"]),
        plan_id=str(value["plan_id"]),
        execution_identity=str(value["execution_identity"]),
    )
    expected = authorize_live_request(authority, request)
    if dict(value) != expected:
        raise PolicyError("delegation value differs from current authority")
    return expected


def validate_canonical_delegation(authority: Authority, raw: bytes) -> dict[str, Any]:
    value = parse_canonical_json(raw)
    if not isinstance(value, Mapping):
        raise PolicyError("delegation must be one canonical JSON object")
    return validate_delegation(authority, value)

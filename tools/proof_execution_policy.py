#!/usr/bin/env python3
"""Execution-class policy for Linux VST Bridge proof tooling.

This module is intentionally independent from product/VST code. It turns the
human authority in CURRENT_SLICE.md into a small, deterministic live-execution
decision. It is an operability guard, not a security sandbox.
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
PLAN_ID = re.compile(r"[a-z0-9][a-z0-9._-]{0,127}")


class PolicyError(RuntimeError):
    """A closed proof-execution policy refusal."""


class ExecutionClass(str, Enum):
    READ_ONLY_RECONCILIATION = "READ_ONLY_RECONCILIATION"
    DIAGNOSTIC_NON_AUTHORITATIVE = "DIAGNOSTIC_NON_AUTHORITATIVE"
    ACCEPTANCE_CANDIDATE = "ACCEPTANCE_CANDIDATE"


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
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True)
            + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


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
        "permitted_execution_class",
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
    return {
        "schema": AUTHORITY_SCHEMA,
        "authority_sha256": authority.raw_sha256,
        "status": authority.fields["status"],
        "authority_phase": authority.fields["authority_phase"],
        "change_class": authority.fields["change_class"],
        "product_implementation_authorized":
            authority.boolean("product_implementation_authorized"),
        "live_execution_authorized": authority.boolean("live_execution_authorized"),
        "permitted_execution_class": permitted,
    }


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
    if authority.fields.get("classified_backend_ready") != "true":
        raise PolicyError("CLASSIFIED_BACKEND_REQUIRED: live backend is not class-aware")

    if request.execution_class is ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE:
        identity_key = "diagnostic_campaign_identity"
        budget_key = "diagnostic_batch_budget"
        maximum = authority.integer(budget_key)
        if maximum not in {1, 2}:
            raise PolicyError("diagnostic batch budget must be one or two")
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
        "acceptance_eligible": acceptance_eligible,
        "batch_budget_maximum": maximum,
    }


def validate_delegation(
    authority: Authority, value: Mapping[str, Any],
) -> dict[str, Any]:
    keys = {
        "schema", "authority_sha256", "execution_class", "execution_identity",
        "source_commit", "plan_id", "acceptance_eligible",
        "batch_budget_maximum",
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

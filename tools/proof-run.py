#!/usr/bin/env python3
"""Classified proof command; explicit scoped authority and actionable failures."""
from __future__ import annotations

import argparse
from dataclasses import asdict
import os
import pathlib
import pwd
import re
import subprocess
import sys
from typing import Callable

from classified_proof_backend import BackendError, ClassifiedProofBackend, PreflightContext
from pc0_proof_adapter import (
    PRODUCTION_ADAPTERS, PLAN_ID, AdapterPorts, OSFileSystemPort,
    StrictSSHPort, SubprocessCommandPort, create_pc0_diagnostic_adapter,
)
from pc0_acceptance_adapter import PLAN_ID as ACCEPTANCE_PLAN_ID, create_pc0_acceptance_adapter
from proof_execution_policy import (
    ExecutionClass, LiveRequest, PolicyError, authority_status,
    authorize_live_request, canonical_json, load_authority,
    validate_canonical_delegation, sha256_bytes,
)

ROOT = pathlib.Path(__file__).resolve().parents[1]
AUTHORITY = ROOT / "CURRENT_SLICE.md"
LOCAL_BACKEND = ROOT / "tools/host-proof.py"


def production_state_root() -> pathlib.Path:
    """The OS account, not environment variables, owns the budget namespace."""
    home = pathlib.Path(pwd.getpwuid(os.getuid()).pw_dir)
    if not home.is_absolute():
        raise BackendError("OS account home is not an absolute path")
    if sys.platform == "darwin":
        return home / "Library" / "Application Support" / "Linux VST Bridge" / "proof" / "classified-proof"
    return home / ".local" / "state" / "linux-vst-bridge" / "classified-proof"


STATE_ROOT = production_state_root()


def safe_detail(value: str | bytes) -> str:
    """Bounded final error line; never emit a traceback, command, or credentials."""
    text = value.decode("utf-8", "replace") if isinstance(value, bytes) else value
    lines = [line.strip() for line in text[-8192:].splitlines() if line.strip()]
    text = lines[-1] if lines else "no error detail returned"
    text = re.sub(r"\b(?:gh[pousr]_[A-Za-z0-9_]+|github_pat_[A-Za-z0-9_]+)\b", "<credential>", text)
    text = re.sub(r"(?i)\b(?:bearer|basic)\s+\S+", "<credential>", text)
    text = re.sub(r"(?i)\b(password|token|secret|authorization|cookie)\b\s*[:=]\s*\S+", r"\1=<redacted>", text)
    text = re.sub(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\b", "<account>", text)
    text = re.sub(r"\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b", "<address>", text)
    text = re.sub(r"(?<![A-Za-z0-9])(?:[A-Za-z]:[\\/]|/)[^\s\"']+", "<path>", text)
    text = re.sub(r"[\x00-\x1f\x7f]", "?", text)
    return text[:768]


def error_chain(error: BaseException) -> str:
    parts, seen = [], set()
    current: BaseException | None = error
    for _ in range(6):
        if current is None or id(current) in seen:
            break
        seen.add(id(current))
        parts.append(f"{type(current).__name__}: {safe_detail(str(current))}")
        current = current.__cause__
    detail = getattr(error, "command_diagnostic", None)
    if isinstance(detail, str):
        parts.append(detail)
    return " <- ".join(parts)[:4096]


class ReportingCommandPort(SubprocessCommandPort):
    """Retain a bounded failed-command explanation without changing its outcome."""
    def __init__(self) -> None:
        self.last_failure: str | None = None

    def run(self, argv, *, cwd=None, timeout=30.0, input_bytes=None):
        self.last_failure = None
        try:
            reply = super().run(argv, cwd=cwd, timeout=timeout, input_bytes=input_bytes)
        except Exception as exc:
            self.last_failure = f"command {pathlib.Path(argv[0]).name}: {safe_detail(str(exc))}"
            raise
        if reply.returncode != 0:
            self.last_failure = (
                f"command {pathlib.Path(argv[0]).name} exit {reply.returncode}: "
                f"{safe_detail(reply.stderr)}"
            )
        return reply


def _production_adapters():
    command = ReportingCommandPort()
    ports = AdapterPorts(command, OSFileSystemPort(), StrictSSHPort(command))
    adapters = dict(PRODUCTION_ADAPTERS)
    adapters[PLAN_ID] = create_pc0_diagnostic_adapter(ports)
    adapters[ACCEPTANCE_PLAN_ID] = create_pc0_acceptance_adapter(ports)
    from ap0_adapter import adapters as ap0_adapters
    adapters.update(ap0_adapters(ports))
    from ap1_adapter import adapters as ap1_adapters
    adapters.update(ap1_adapters(ports))
    from ap2_adapter import adapters as ap2_adapters
    adapters.update(ap2_adapters(ports))
    return adapters, command


def _run_local(operation: str, source: str, plan: str) -> int:
    return subprocess.run(
        [sys.executable, str(LOCAL_BACKEND), operation, "--source", source, "--plan", plan],
        cwd=ROOT, check=False,
    ).returncode


def _live_request(args: argparse.Namespace) -> LiveRequest:
    diagnostic = args.operation == "diagnose"
    return LiveRequest(
        execution_class=ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE if diagnostic else ExecutionClass.ACCEPTANCE_CANDIDATE,
        source_commit=args.source, plan_id=args.plan,
        execution_identity=args.campaign if diagnostic else args.candidate,
    )


def dispatch(args: argparse.Namespace, *,
             local_runner: Callable[[str, str, str], int] = _run_local,
             classified_runner: Callable[[object, bytes], object] | None = None) -> int:
    authority_path = pathlib.Path(getattr(args, "authority", None) or AUTHORITY)
    authority = load_authority(authority_path)
    if args.operation == "status":
        print(canonical_json(authority_status(authority)).decode(), end="")
        return 0
    if args.operation in {"plan", "validate"}:
        return local_runner(args.operation, args.source, args.plan)

    delegation = authorize_live_request(authority, _live_request(args))
    encoded = canonical_json(delegation)
    if getattr(args, "preflight_only", False):
        # Deliberately no backend constructor, execute(), or budget state path.
        current = load_authority(authority_path)
        if current.raw_sha256 != authority.raw_sha256:
            raise BackendError("authority changed before read-only preflight")
        validate_canonical_delegation(current, encoded)
        adapters, command = _production_adapters()
        adapter = adapters.get(args.plan)
        if adapter is None:
            raise BackendError("requested plan has no production adapter")
        try:
            adapter.preflight(PreflightContext(
                delegation=delegation, plan_descriptor=adapter.descriptor.record()))
        except Exception as exc:
            exc.command_diagnostic = command.last_failure
            raise BackendError(error_chain(exc)) from exc
        output = {"operation": "read_only_preflight", "state": "ready",
                  "reservations_created": 0, "acceptance_eligible": False}
    elif getattr(args, "render_only", False):
        if args.operation != "accept":
            raise BackendError("retained rendering is acceptance-only")
        adapters, _command = _production_adapters()
        receipt = ClassifiedProofBackend(STATE_ROOT, adapters).render_retained(authority, encoded,
            renderer_revision=sha256_bytes((ROOT / "tools/wf0-factory-census/evidence.py").read_bytes()))
        output = asdict(receipt)
    elif classified_runner is not None:
        receipt = classified_runner(authority, encoded)
        output = asdict(receipt) if hasattr(receipt, "__dataclass_fields__") else receipt
    else:
        adapters, command = _production_adapters()
        try:
            receipt = ClassifiedProofBackend(STATE_ROOT, adapters).execute(authority, encoded)
        except Exception as exc:
            exc.command_diagnostic = command.last_failure
            raise
        output = asdict(receipt) if hasattr(receipt, "__dataclass_fields__") else receipt
    if not isinstance(output, dict):
        raise BackendError("classified backend returned no receipt")
    print(canonical_json(output).decode(), end="")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="operation", required=True)
    status = sub.add_parser("status")
    status.add_argument("--authority", type=pathlib.Path)
    for name in ("plan", "validate"):
        item = sub.add_parser(name)
        item.add_argument("--source", required=True)
        item.add_argument("--plan", required=True)
    for name, identity in (("diagnose", "campaign"), ("accept", "candidate")):
        item = sub.add_parser(name)
        item.add_argument("--source", required=True)
        item.add_argument("--plan", required=True)
        item.add_argument(f"--{identity}", required=True)
        item.add_argument("--authority", type=pathlib.Path,
                          help="Explicit scoped receipt; default CURRENT_SLICE remains fail-closed")
        if name == "accept":
            item.add_argument("--render-only", action="store_true",
                              help="Render the immutable retained acceptance result; no remote calls")
        item.add_argument("--preflight-only", action="store_true",
                          help="Read-only adapter checks only; never reserve or launch")
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        return dispatch(args)
    except (PolicyError, BackendError, OSError) as error:
        print(error_chain(error), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Classified proof command for Linux VST Bridge.

This command is the only supported future entry point for workload-producing
proof operations. PX2 routes a validated canonical delegation into the
class-aware core. The production adapter registry remains empty, so this
module cannot currently launch a workload.
"""

from __future__ import annotations

import argparse
from dataclasses import asdict
import os
import pathlib
import pwd
import subprocess
import sys
from typing import Callable

from classified_proof_backend import (
    BackendError, ClassifiedProofBackend, PRODUCTION_ADAPTERS,
)
from proof_execution_policy import (
    ExecutionClass, LiveRequest, PolicyError, authority_status,
    authorize_live_request, canonical_json, load_authority,
)


ROOT = pathlib.Path(__file__).resolve().parents[1]
AUTHORITY = ROOT / "CURRENT_SLICE.md"
LOCAL_BACKEND = ROOT / "tools/host-proof.py"


def production_state_root() -> pathlib.Path:
    """Use the OS account home, never caller-controlled environment paths."""
    real_home = pathlib.Path(pwd.getpwuid(os.getuid()).pw_dir)
    if not real_home.is_absolute():
        raise BackendError("OS account home is not an absolute path")
    if sys.platform == "darwin":
        return (
            real_home / "Library" / "Application Support" / "Linux VST Bridge"
            / "proof" / "classified-proof"
        )
    return real_home / ".local" / "state" / "linux-vst-bridge" / "classified-proof"


STATE_ROOT = production_state_root()


def _run_local(operation: str, source: str, plan: str) -> int:
    result = subprocess.run(
        [sys.executable, str(LOCAL_BACKEND), operation,
         "--source", source, "--plan", plan],
        cwd=ROOT,
        check=False,
    )
    return result.returncode


def _live_request(args: argparse.Namespace) -> LiveRequest:
    if args.operation == "diagnose":
        execution_class = ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE
        identity = args.campaign
    else:
        execution_class = ExecutionClass.ACCEPTANCE_CANDIDATE
        identity = args.candidate
    return LiveRequest(
        execution_class=execution_class,
        source_commit=args.source,
        plan_id=args.plan,
        execution_identity=identity,
    )


def dispatch(
    args: argparse.Namespace,
    *,
    local_runner: Callable[[str, str, str], int] = _run_local,
    classified_runner: Callable[[object, bytes], object] | None = None,
) -> int:
    authority = load_authority(AUTHORITY)
    if args.operation == "status":
        print(canonical_json(authority_status(authority)).decode(), end="")
        return 0
    if args.operation in {"plan", "validate"}:
        return local_runner(args.operation, args.source, args.plan)

    request = _live_request(args)
    delegation = authorize_live_request(authority, request)
    delegation_bytes = canonical_json(delegation)
    if classified_runner is None:
        backend = ClassifiedProofBackend(STATE_ROOT, PRODUCTION_ADAPTERS)
        receipt = backend.execute(authority, delegation_bytes)
    else:
        receipt = classified_runner(authority, delegation_bytes)
    if hasattr(receipt, "__dataclass_fields__"):
        output = asdict(receipt)
    elif isinstance(receipt, dict):
        output = receipt
    else:
        raise BackendError("classified backend returned no receipt")
    print(canonical_json(output).decode(), end="")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="operation", required=True)
    sub.add_parser("status")
    for name in ("plan", "validate"):
        item = sub.add_parser(name)
        item.add_argument("--source", required=True)
        item.add_argument("--plan", required=True)
    diagnostic = sub.add_parser("diagnose")
    diagnostic.add_argument("--source", required=True)
    diagnostic.add_argument("--plan", required=True)
    diagnostic.add_argument("--campaign", required=True)
    acceptance = sub.add_parser("accept")
    acceptance.add_argument("--source", required=True)
    acceptance.add_argument("--plan", required=True)
    acceptance.add_argument("--candidate", required=True)
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        return dispatch(args)
    except (PolicyError, BackendError) as error:
        print(str(error), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())

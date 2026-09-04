#!/usr/bin/env python3
"""Classified proof command for Linux VST Bridge.

This command is the only supported future entry point for workload-producing
proof operations. PX1 intentionally blocks live delegation until a backend
records execution class, identity, eligibility, and budget in its result.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import subprocess
import sys
from typing import Callable

from proof_execution_policy import (
    ExecutionClass, LiveRequest, PolicyError, authority_status,
    authorize_live_request, canonical_json, load_authority,
)


ROOT = pathlib.Path(__file__).resolve().parents[1]
AUTHORITY = ROOT / "CURRENT_SLICE.md"
LOCAL_BACKEND = ROOT / "tools/host-proof.py"


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
) -> int:
    authority = load_authority(AUTHORITY)
    if args.operation == "status":
        print(canonical_json(authority_status(authority)).decode(), end="")
        return 0
    if args.operation in {"plan", "validate"}:
        return local_runner(args.operation, args.source, args.plan)

    request = _live_request(args)
    delegation = authorize_live_request(authority, request)

    # A class-aware live backend must persist this exact delegation in its
    # transaction/result before PX1 can enable delegation. The old DX0 backend
    # cannot do that and would allow diagnostic promotion, so fail closed.
    raise PolicyError(
        "CLASSIFIED_BACKEND_REQUIRED: authority is valid, but no backend "
        "currently records execution_class, execution_identity, "
        "acceptance_eligible, and separate diagnostic/acceptance budgets; "
        f"validated delegation={json.dumps(delegation, sort_keys=True)}"
    )


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
    except PolicyError as error:
        print(str(error), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())

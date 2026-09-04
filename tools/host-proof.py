#!/usr/bin/env python3
"""Policy-gated entry point for the accepted DX0 proof driver.

Local ``plan`` and ``validate`` remain available. Every workload-producing
operation is blocked here until invoked by a class-aware backend. The accepted
legacy implementation is loaded by exact historical Git blob only for
local-only operations; no executable legacy path exists in the worktree.
"""

from __future__ import annotations

import pathlib
import subprocess
import sys
import types


LEGACY_LOCAL_ONLY_BLOB = "beb57c62b94f3d99fb87f56db3b20b20553064c4"
LOCAL_OPERATIONS = {"plan", "validate"}
BLOCKED_LIVE_OPERATIONS = {"run", "seed-fixture"}


def _usage() -> str:
    return (
        "usage: python3 tools/host-proof.py {plan|validate} ...\n"
        "live execution: use python3 tools/proof-run.py; direct run and "
        "seed-fixture are forbidden\n"
    )


def _load_legacy_local_module() -> types.ModuleType:
    root = pathlib.Path(__file__).resolve().parents[1]
    result = subprocess.run(
        ["git", "cat-file", "blob", LEGACY_LOCAL_ONLY_BLOB],
        cwd=root,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
        timeout=30,
    )
    if result.returncode != 0 or not result.stdout:
        raise RuntimeError("LOCAL_PROOF_BACKEND_BLOCKED: exact legacy blob is unavailable")
    observed = subprocess.run(
        ["git", "hash-object", "--stdin"],
        cwd=root,
        input=result.stdout,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
        timeout=30,
    )
    if (observed.returncode != 0
            or observed.stdout.decode("ascii", "strict").strip()
            != LEGACY_LOCAL_ONLY_BLOB):
        raise RuntimeError("LOCAL_PROOF_BACKEND_BLOCKED: legacy blob readback differs")
    module = types.ModuleType("_linux_vst_bridge_legacy_host_proof")
    module.__file__ = str(pathlib.Path(__file__).resolve())
    module.__package__ = None
    exec(compile(result.stdout, module.__file__, "exec"), module.__dict__)
    return module


def main(argv: list[str] | None = None) -> int:
    args = list(sys.argv[1:] if argv is None else argv)
    if not args or args[0] in {"-h", "--help"}:
        print(_usage(), end="")
        return 0
    operation = args[0]
    if operation in BLOCKED_LIVE_OPERATIONS:
        print(
            "UNCLASSIFIED_LIVE_EXECUTION_FORBIDDEN: "
            "use tools/proof-run.py with an exact execution class and authority",
            file=sys.stderr,
        )
        return 2
    if operation not in LOCAL_OPERATIONS:
        print(_usage(), file=sys.stderr, end="")
        return 2
    module = _load_legacy_local_module()
    old_argv = sys.argv
    try:
        sys.argv = [str(pathlib.Path(__file__).resolve()), *args]
        return int(module.main())
    finally:
        sys.argv = old_argv


if __name__ == "__main__":
    raise SystemExit(main())

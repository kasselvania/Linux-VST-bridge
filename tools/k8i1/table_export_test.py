#!/usr/bin/env python3
"""Exercise the production table exporter against the source-owned MSI fixture."""
from __future__ import annotations

import pathlib
import subprocess
import sys
import tempfile


def decode(data: bytes) -> str:
    if data.startswith((b"\xff\xfe", b"\xfe\xff")):
        return data.decode("utf-16")
    return data.decode("utf-8", errors="strict")


def main() -> None:
    if len(sys.argv) != 3:
        raise SystemExit("usage: table_export_test.py EXPORTER FIXTURE_MSI")
    exporter, msi = (pathlib.Path(value).resolve() for value in sys.argv[1:])
    with tempfile.TemporaryDirectory(prefix="k8i1-table-export-") as raw:
        output = pathlib.Path(raw).resolve()
        subprocess.run([str(exporter), str(msi), str(output)], check=True, timeout=30)
        roster = [line for line in decode((output / "_k8i1-tables.txt").read_bytes()).splitlines() if line]
        if roster != sorted(set(roster)) or not {"_Tables", "Property", "InstallExecuteSequence"} <= set(roster):
            raise AssertionError(("table_roster", roster))
        projected = sorted(path.stem for path in output.glob("*.idt"))
        if projected != roster:
            raise AssertionError(("projection_roster", projected, roster))
        if any(not path.read_bytes() for path in output.glob("*.idt")):
            raise AssertionError("empty_projection")
    print(f"K8I1_TABLE_EXPORT_FIXTURE_V1 tables={len(roster)} complete=1")


if __name__ == "__main__":
    main()

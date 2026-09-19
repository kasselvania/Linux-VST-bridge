#!/usr/bin/env python3
"""Generate the exact Wine MSI forwarder definition from the pinned roster."""
from __future__ import annotations

import argparse
import pathlib
import re

INTERCEPTED = {"MsiInstallProductA", "MsiInstallProductW"}
STDCALL_BYTES = {"MsiInstallProductA": 8, "MsiInstallProductW": 8}


def parse(path: pathlib.Path) -> list[tuple[int, str]]:
    rows: list[tuple[int, str]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line:
            continue
        match = re.fullmatch(r"([0-9]{1,3})\t([A-Za-z_][A-Za-z0-9_]*)", line)
        if not match:
            raise ValueError("k8i1_export_roster_shape")
        rows.append((int(match[1]), match[2]))
    if len(rows) != 296 or [ordinal for ordinal, _ in rows] != list(range(5, 301)):
        raise ValueError("k8i1_export_roster_extent")
    if {name for _, name in rows if name in INTERCEPTED} != INTERCEPTED:
        raise ValueError("k8i1_intercept_exports_missing")
    if len({name for _, name in rows}) != len(rows):
        raise ValueError("k8i1_export_roster_duplicate")
    return rows


def definition(rows: list[tuple[int, str]]) -> str:
    lines = ["LIBRARY msi.dll", "EXPORTS"]
    for ordinal, name in rows:
        if name in INTERCEPTED:
            # MSVC's x86 WINAPI definitions are stdcall-decorated internally;
            # the public export remains the exact undecorated Wine name.
            lines.append(f"{name}=_{name}@{STDCALL_BYTES[name]} @{ordinal}")
        else:
            lines.append(f"{name}=msi_lvb_real.{name} @{ordinal}")
    return "\n".join(lines) + "\n"


def real_definition(rows: list[tuple[int, str]]) -> str:
    """Describe the pinned real DLL so MSVC can resolve mixed forwarders."""
    lines = ["LIBRARY msi_lvb_real.dll", "EXPORTS"]
    lines.extend(f"{name} @{ordinal}" for ordinal, name in rows)
    return "\n".join(lines) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--roster", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    parser.add_argument("--real-output", type=pathlib.Path, required=True)
    args = parser.parse_args()
    rows = parse(args.roster)
    args.output.write_text(definition(rows), encoding="ascii")
    args.real_output.write_text(real_definition(rows), encoding="ascii")


if __name__ == "__main__":
    main()

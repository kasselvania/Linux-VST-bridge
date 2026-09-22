#!/usr/bin/env python3
"""Fail-closed validation of the public RPI1 transfer baseline."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


class BaselineError(RuntimeError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise BaselineError(message)


def digest(path: Path) -> str:
    require(path.is_file() and not path.is_symlink(), f"not an exact file: {path}")
    value = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(65536), b""):
            value.update(block)
    return value.hexdigest()


def read_json(path: Path) -> Any:
    with path.open("rb") as source:
        return json.load(source)


def validate(root: Path, manifest_path: Path) -> dict[str, Any]:
    root = root.resolve(strict=True)
    manifest_path = manifest_path.resolve(strict=True)
    baseline = read_json(manifest_path)
    require(
        baseline.get("schema") == "linux-vst-bridge-rpi1-transfer-baseline/v1",
        "transfer schema",
    )
    require(
        baseline.get("status")
        == "TRANSFER_BASELINE_RESOLVED_PI_PREFLIGHT_PASSED_WITH_PRESERVED_FAULTS",
        "transfer status",
    )

    for record in baseline.get("repository_records", []):
        relative = Path(record["path"])
        require(not relative.is_absolute() and ".." not in relative.parts, "record path")
        require(digest(root / relative) == record["sha256"], f"record digest: {relative}")

    profile = read_json(root / "compatibility/arturia-pigments.json")
    pigments = baseline["pigments"]
    require(profile["id"] == pigments["profile_id"], "Pigments profile id")
    require(profile["revision"] == pigments["profile_revision"], "Pigments revision")
    require(profile["claim"] == pigments["claim"], "Pigments claim")
    require(profile["module_sha256"] == pigments["module_sha256"], "Pigments module")
    require(profile["class"]["class_id"] == pigments["class_id"], "Pigments class")
    require(profile["class"]["version"] == pigments["version"], "Pigments version")
    require(
        profile["requirements"]["environment_family"] == pigments["environment_family"],
        "Arturia environment family",
    )
    require(
        profile["requirements"]["environment_revision"]
        == pigments["environment_revision"],
        "Arturia environment revision",
    )
    require(profile["capabilities"] == pigments["capabilities"], "Pigments capabilities")

    runner = profile["requirements"]["runner"]
    expected_runner = baseline["deck_runner"]
    for key in ("id", "version", "proton_sha256", "entry_point_sha256"):
        require(runner[key] == expected_runner[key], f"runner {key}")
    require(
        len(runner["file_sha256"]) == 31
        and len(set(runner["file_sha256"])) == 31
        and all(len(value) == 64 for value in runner["file_sha256"]),
        "runner closure",
    )

    installation = read_json(root / "evidence/ap18/asc-installation.json")
    asc = baseline["asc"]
    require(
        installation["installer"]["sha256"] == asc["known_installer_sha256"],
        "ASC installer identity",
    )
    executables = {
        Path(item["relative"]).name: item["sha256"]
        for item in installation["installation"]["executables"]
    }
    require(
        executables["Arturia Software Center.exe"] == asc["known_main_sha256"],
        "ASC main identity",
    )
    require(
        executables["ArturiaSoftwareCenterAgent.exe"] == asc["known_agent_sha256"],
        "ASC Agent identity",
    )
    require(executables["updater.exe"] == asc["known_updater_sha256"], "ASC updater")

    attribution = read_json(root / "evidence/ap18/post-login/uia-attribution.json")
    require(
        attribution["account_free_reproduction"]["operation_local_override"]
        ["WINEDLLOVERRIDES"]
        == asc["process_local_environment"]["WINEDLLOVERRIDES"],
        "ASC accessibility policy",
    )

    uia = read_json(root / "evidence/rpi1/account-free-uia-preflight.json")
    require(
        uia["schema"] == "linux-vst-bridge-rpi1-account-free-uia-preflight/v1",
        "RPI1 UIA schema",
    )
    package = read_json(root / "tools/uio2/package.json")
    require(
        uia["source_owned_windows_fixture"]["executable_sha256"]
        == package["files"]["uio2-uia-disconnect.exe"],
        "RPI1 UIA fixture",
    )
    require(
        uia["comparison"]["baseline"]["expected_exit"]
        == uia["comparison"]["baseline"]["observed_exit"]
        == 42,
        "RPI1 UIA baseline",
    )
    require(
        uia["comparison"]["operation_local_override"]["WINEDLLOVERRIDES"]
        == asc["process_local_environment"]["WINEDLLOVERRIDES"]
        and uia["comparison"]["operation_local_override"]["expected_exit"]
        == uia["comparison"]["operation_local_override"]["observed_exit"]
        == 0,
        "RPI1 UIA override",
    )
    require(
        uia["ownership_and_cleanup"]["result"] == "PASSED"
        and uia["gates"]["account_free_uia_normal_and_override"] == "PASSED",
        "RPI1 UIA cleanup",
    )
    require(
        baseline["gates"]["account_free_uia"] == "PASSED"
        and baseline["gates"]["proton_on_box64_preflight"]
        == "PASSED_WITH_PRESERVED_SETUP_FAULTS",
        "RPI1 preflight gates",
    )

    rpi0 = read_json(
        root / "evidence/rpi0-standalone-arm64-appliance/physical-acceptance.json"
    )
    source = baseline["starting_source"]
    require(
        rpi0["custody"]["physical_defect_repair_source_head"]
        == source["rpi0_executable_head"],
        "RPI0 executable head",
    )
    require(
        rpi0["custody"]["physical_defect_repair_source_tree"]
        == source["rpi0_executable_tree"],
        "RPI0 executable tree",
    )
    require(
        rpi0["translation"]["box64_commit"]
        == baseline["pi_translation"]["box64_commit"],
        "Box64 source",
    )
    require(
        rpi0["translation"]["box64_sha256"]
        == baseline["pi_translation"]["box64_sha256"],
        "Box64 executable",
    )
    return baseline


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument(
        "--manifest",
        type=Path,
        default=Path(__file__).with_name("transfer-baseline.json"),
    )
    arguments = parser.parse_args()
    try:
        baseline = validate(arguments.root, arguments.manifest)
    except (BaselineError, KeyError, OSError, ValueError, json.JSONDecodeError) as error:
        print(f"RPI1_TRANSFER_BASELINE_REFUSED {error}")
        return 1
    print(
        "RPI1_TRANSFER_BASELINE_PASS "
        f"profile_revision={baseline['pigments']['profile_revision']} "
        f"runner={baseline['deck_runner']['id']} "
        f"starting_head={baseline['starting_source']['head']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

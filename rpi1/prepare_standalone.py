#!/usr/bin/env python3
"""Create a private RPI1 standalone config without copying licensed payloads."""
import argparse
import hashlib
import os
from pathlib import Path

PINS = {
    "box64": "79cdd30e5480f5dfb8cd717af99d0e5a89eb55b31701de6fde7896a6a0e79ab6",
    "adapter": "99e1dc907c53fba755c1839922654753c09435fa3c474ded00f8dc2d499b7828",
    "emulator": "9a30bd4f6cd6ec7e888a90af81f26aa9c3967abc9410e3535dea7a804f18e7c1",
    "graphics": "a076eea9d36c398b94b9a9b1f093d31811659a731169a42bf9ad6bae54bf871d",
    "slr": "caa39b5cde8ea955288b574b49b3416fd16e7be3f53a17249d673f5ecfdce2c1",
    "proton": "787504a79bacf6b303984a9a846cf47463f36599248e8306b26d5faf78267aad",
    "pigments": "bdc91ebef8e5b486c8f998f1eef6a99626dd5a0b46d986263eeb1980b96a3c07",
}


def exact_file(value: Path, label: str) -> Path:
    if not value.is_absolute() or value.is_symlink() or not value.is_file():
        raise ValueError(f"{label} is not an absolute non-symlink regular file")
    return value


def exact_directory(value: Path, label: str) -> Path:
    if not value.is_absolute() or value.is_symlink() or not value.is_dir():
        raise ValueError(f"{label} is not an absolute non-symlink directory")
    if value.stat().st_uid != os.getuid():
        raise ValueError(f"{label} is not owned by the invoking user")
    return value


def digest(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(65536), b""):
            value.update(block)
    return value.hexdigest()


def pinned(path: Path, expected: str, label: str) -> Path:
    exact_file(path, label)
    if digest(path) != expected:
        raise ValueError(f"{label} SHA-256 differs")
    return path


def sha256(value: str) -> str:
    if len(value) != 64 or any(character not in "0123456789abcdefABCDEF" for character in value):
        raise ValueError("source manifest SHA-256 syntax")
    return value.lower()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--environment-root", required=True, type=Path)
    parser.add_argument("--runtime-variable-dir", required=True, type=Path)
    parser.add_argument("--windows-host", required=True, type=Path)
    parser.add_argument("--host-source-manifest-sha256", required=True)
    parser.add_argument("--plugin", required=True, type=Path)
    parser.add_argument("--xauthority", required=True, type=Path)
    parser.add_argument("--display", default=":1")
    parser.add_argument("--bridge-frames", choices=("512", "1024", "2048"), default="2048")
    parser.add_argument("--output", required=True, type=Path)
    arguments = parser.parse_args()

    root = exact_directory(arguments.environment_root, "environment root")
    variable = exact_directory(arguments.runtime_variable_dir, "runtime variable directory")
    if variable.parent != root or not variable.name.startswith("runtime-var-pyhome-") or not (variable / ".ref").is_file():
        raise ValueError("runtime variable directory binding differs")
    if not arguments.output.is_absolute() or arguments.output.exists():
        raise ValueError("output must be an absent absolute path")
    if not arguments.display or any(character in "\0\r\n" for character in arguments.display):
        raise ValueError("display identity")

    files = {
        "box64": pinned(root / "emulator/bin/box64", PINS["box64"], "Box64"),
        "adapter": pinned(root / "emulator/adapter", PINS["adapter"], "emulator adapter"),
        "emulator": pinned(root / "emulator/emulator.json", PINS["emulator"], "emulator manifest"),
        "graphics": pinned(root / "emulator/graphics-provider.json", PINS["graphics"], "graphics manifest"),
        "slr": pinned(root / "runtime/SteamLinuxRuntime_4/_v2-entry-point", PINS["slr"], "SLR4 entry"),
        "proton": pinned(root / "runtime/Proton 11.0/proton", PINS["proton"], "Proton"),
        "host": exact_file(arguments.windows_host, "Windows host"),
        "plugin": pinned(arguments.plugin, PINS["pigments"], "Pigments"),
        "xauthority": exact_file(arguments.xauthority, "X authority"),
    }
    drive_c = root / "compatdata/pfx/drive_c"
    if not files["host"].is_relative_to(drive_c) or not files["plugin"].is_relative_to(drive_c):
        raise ValueError("Windows artifact is outside selected C: drive")
    evidence = root / "standalone-evidence"
    evidence.mkdir(mode=0o700, exist_ok=True)
    if evidence.is_symlink() or evidence.stat().st_uid != os.getuid():
        raise ValueError("evidence directory identity")

    values = {
        "runtime_id": "proton-11.0-2c-25118279-slr4-4.0.20260805.254769",
        "box64_source_commit": "2f130fab1d6e1a4ee8a71dc60cfdfcc839ad192a",
        "box64_path": files["box64"], "box64_sha256": PINS["box64"],
        "emulator_adapter_path": files["adapter"], "emulator_adapter_sha256": PINS["adapter"],
        "emulator_manifest_path": files["emulator"], "emulator_manifest_sha256": PINS["emulator"],
        "graphics_manifest_path": files["graphics"], "graphics_manifest_sha256": PINS["graphics"],
        "slr_entry_path": files["slr"], "slr_entry_sha256": PINS["slr"],
        "proton_path": files["proton"], "proton_sha256": PINS["proton"],
        "windows_host_path": files["host"], "windows_host_sha256": digest(files["host"]),
        "plugin_path": files["plugin"], "plugin_sha256": PINS["pigments"],
        "environment_root": root, "runtime_variable_dir": variable,
        "display": arguments.display,
        "xauthority_path": files["xauthority"], "xauthority_sha256": digest(files["xauthority"]),
        "source_manifest_sha256": sha256(arguments.host_source_manifest_sha256),
        "bridge_frames": arguments.bridge_frames, "jack_client": "lvb-arm-pigments",
        "evidence_directory": evidence, "protocol_minor": "12", "sample_rate": "48000",
        "pythonhome": "/usr", "machine_architecture": "aarch64-linux-gnu",
        "accessibility_policy": "uiautomationcore=",
        "event_output_policy": "reported_zero_event_channels_unspecified",
        "editor_lifetime_policy": "retain_editor_view_until_instance_retirement",
        "vendor_retirement_policy": "process_scoped_vendor_retirement",
        "pigments_class_id": "41727475415649534B61743150726F63",
        "pigments_version": "7.0.1.6772", "pigments_parameter_count": "4446",
        "pigments_precision": "float32_only", "environment_family": "arturia_persistent_v1:1",
    }
    fd = os.open(arguments.output, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, "w", encoding="utf-8") as output:
        output.write("".join(f"{key}={value}\n" for key, value in values.items()))
        output.flush()
        os.fsync(output.fileno())
    print(arguments.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Stage an exact, private RPI0 runtime without downloading any dependency."""
import argparse
import hashlib
import os
from pathlib import Path
import shutil
import subprocess

BOX64_COMMIT = "2f130fab1d6e1a4ee8a71dc60cfdfcc839ad192a"
WINE_COMMIT = "db11d0fe6a169c457e23d007e20404643d067aa8"

def exact_file(value: str) -> Path:
    path = Path(value)
    if not path.is_absolute() or path.is_symlink() or not path.is_file():
        raise argparse.ArgumentTypeError("expected an absolute, non-symlink regular file")
    return path

def digest(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(65536), b""):
            value.update(block)
    return value.hexdigest()

def copy(source: Path, destination: Path, executable: bool = False) -> Path:
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, destination)
    destination.chmod(0o755 if executable else 0o644)
    return destination

def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--box64", required=True, type=exact_file)
    parser.add_argument("--wine64", required=True, type=exact_file)
    parser.add_argument("--linux-probe", required=True, type=exact_file)
    parser.add_argument("--windows-probe", required=True, type=exact_file)
    parser.add_argument("--windows-host", required=True, type=exact_file)
    parser.add_argument("--plugin", required=True, type=exact_file)
    parser.add_argument("--bridge-frames", choices=("512", "1024", "2048"), default="2048")
    arguments = parser.parse_args()
    output = arguments.output
    if not output.is_absolute() or output.exists():
        parser.error("--output must be an absent absolute path")
    output.mkdir(mode=0o700)
    root = Path(__file__).resolve().parents[1]
    staged = {
        "box64": copy(arguments.box64, output / "box64/bin/box64", True),
        "wine": copy(arguments.wine64, output / "wine/bin/wine64", True),
        "linux_probe": copy(arguments.linux_probe, output / "probes/rpi0-linux-probe", True),
        "windows_probe": copy(arguments.windows_probe, output / "prefix/drive_c/bridge/probes/rpi0-windows-probe.exe"),
        "windows_host": copy(arguments.windows_host, output / "prefix/drive_c/bridge/bin/wf0-factory-probe.exe"),
        "plugin": copy(arguments.plugin, output / "prefix/drive_c/bridge/fixtures/LVB ARM Appliance Synth.vst3"),
        "box64_rc": copy(root / "rpi0/box64rc", output / "box64rc"),
    }
    (output / "prefix/drive_c/bridge/sessions").mkdir(parents=True, mode=0o700)
    evidence = output / "evidence"
    evidence.mkdir(mode=0o700)
    head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
    tree = subprocess.check_output(["git", "rev-parse", "HEAD^{tree}"], cwd=root, text=True).strip()
    manifest = output / "source-manifest.txt"
    manifest.write_text(
        "schema=linux-vst-bridge-rpi0-source-manifest/v1\n"
        f"head={head}\ntree={tree}\nbox64_commit={BOX64_COMMIT}\nwine_commit={WINE_COMMIT}\n"
        + "".join(f"{name}_sha256={digest(path)}\n" for name, path in staged.items()),
        encoding="utf-8",
    )
    manifest.chmod(0o600)
    config = output / "appliance.conf"
    config.write_text(
        f"box64_source_commit={BOX64_COMMIT}\nbox64_path={staged['box64']}\nbox64_sha256={digest(staged['box64'])}\n"
        f"wine_source_commit={WINE_COMMIT}\nwine_path={staged['wine']}\nwine_sha256={digest(staged['wine'])}\n"
        f"linux_probe_path={staged['linux_probe']}\nlinux_probe_sha256={digest(staged['linux_probe'])}\n"
        f"windows_probe_path={staged['windows_probe']}\nwindows_probe_sha256={digest(staged['windows_probe'])}\n"
        f"windows_host_path={staged['windows_host']}\nwindows_host_sha256={digest(staged['windows_host'])}\n"
        f"plugin_path={staged['plugin']}\nplugin_sha256={digest(staged['plugin'])}\n"
        f"prefix_path={output / 'prefix'}\nbox64_rc_path={staged['box64_rc']}\nbox64_rc_sha256={digest(staged['box64_rc'])}\n"
        f"source_manifest_sha256={digest(manifest)}\nbridge_frames={arguments.bridge_frames}\njack_client=lvb-arm-standalone\n"
        f"evidence_path={evidence / 'session-01.jsonl'}\nwindows_evidence_path={evidence / 'session-01-windows.jsonl'}\n"
        "protocol_minor=12\nsample_rate=48000\n",
        encoding="utf-8",
    )
    config.chmod(0o600)
    print(config)
    return 0

if __name__ == "__main__":
    raise SystemExit(main())

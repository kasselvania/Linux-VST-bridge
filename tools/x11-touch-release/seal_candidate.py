#!/usr/bin/env python3
"""Compose and seal the exact private Proton successor; never touch the installed runner."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess


RUNNER_ID = "proton-11.0-2c-x11-touch-release-v1"
RUNNER_VERSION = "1788504981 proton-11.0-2c-x86_64+x11-touch-release-v1; SLR 4.0.20260805.254769"
PROTON_SOURCE = "5b89db940e0ebe3a137a6009a3589232fe084c09"
WINE_SOURCE = "dc26e61847081a1b5cb0733dc30feba6ee575482"
WINE_TREE = "da4b1eb3b7f209eb4a971d4b5929fcda08ff6b4e"
PATCH_SHA = "3e8fa75ddb3f1dcfa2b7f73a82d97eeaf8bed0a0d3d51eed6afd514ab3b0723a"
SDK_SHA = "97526b794ce1a9bed5f891084462260b3a02399569f7438a3a57b5a253001db9"
PATCHED_MOUSE_SHA = "d7a76c5d9769f2abb4eefed56293d1ce83fbbce87eab2ca9d4fef4316cc43131"
TOUCH_HEADER_SHA = "ff549851137e2ec4eaacbdb1960c5b25771bafe519cbe008da702b9b261f73a9"


def sha(path):
    h = hashlib.sha256()
    with path.open("rb") as source:
        while block := source.read(1024 * 1024):
            h.update(block)
    return h.hexdigest()


def write_json(path, value):
    data = (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o400)
    with os.fdopen(descriptor, "wb") as output:
        output.write(data)
        output.flush()
        os.fsync(output.fileno())


def tree_identity(root):
    rows = []
    total = 0
    pending = [root]
    while pending:
        directory = pending.pop()
        for path in directory.iterdir():
            metadata = path.lstat()
            relative = path.relative_to(root).as_posix()
            mode = metadata.st_mode & 0o7777
            if stat.S_ISDIR(metadata.st_mode):
                pending.append(path)
                row = ["d", relative, mode]
            elif stat.S_ISREG(metadata.st_mode):
                total += metadata.st_size
                row = ["f", relative, mode, metadata.st_size, sha(path)]
            elif stat.S_ISLNK(metadata.st_mode):
                if not path.resolve(strict=True).is_relative_to(root):
                    raise RuntimeError(f"outside_link:{relative}")
                row = ["l", relative, mode, os.readlink(path)]
            else:
                raise RuntimeError(f"unsupported_type:{relative}")
            rows.append((relative, row))
    rows.sort(key=lambda item: item[0])
    h = hashlib.sha256()
    for _, row in rows:
        h.update(json.dumps(row, separators=(",", ":"), ensure_ascii=False).encode())
        h.update(b"\n")
    return {"schema": 1, "entries": len(rows), "regular_bytes": total, "sha256": h.hexdigest()}


def artifact(path):
    return {"path": str(path), "sha256": sha(path)}


def compose(args, base):
    root = args.root
    old = Path(base["runner"]["proton"]).parent
    assert base["id"] == "4db060b14388e41103834fc4dfdd023a"
    assert base["revision"] == 1
    assert base["runner"]["id"] == "proton-11.0-2c-25118279-slr4-4.0.20260805.254769"
    assert sha(args.patch) == PATCH_SHA
    assert sha(args.source / "dlls/winex11.drv/mouse.c") == PATCHED_MOUSE_SHA
    assert sha(args.source / "dlls/winex11.drv/touch_message_flags.h") == TOUCH_HEADER_SHA
    assert subprocess.check_output(["git", "-C", str(args.source), "rev-parse", "HEAD"], text=True).strip() == WINE_SOURCE
    assert subprocess.check_output(["git", "-C", str(args.source), "rev-parse", "HEAD^{tree}"], text=True).strip() == WINE_TREE
    if root.exists() or root.is_symlink():
        raise RuntimeError("candidate_root_already_exists")
    shutil.copytree(old, root, symlinks=True)
    root.chmod(0o700)
    version = root / "version"
    version.write_text("1788504981 proton-11.0-2c-x86_64+x11-touch-release-v1\n")
    changed = {"version": artifact(version)}
    if (args.stage / "lib/wine/i386-unix/winex11.so").exists():
        raise RuntimeError("unexpected_i386_unix_driver")
    for arch in ("x86_64",):
        relative = f"files/lib/wine/{arch}-unix/winex11.so"
        source = args.stage / "lib" / "wine" / f"{arch}-unix" / "winex11.so"
        target = root / relative
        if not source.is_file() or not target.is_file():
            raise RuntimeError(f"missing_driver:{arch}")
        target.chmod(target.stat().st_mode | stat.S_IWUSR)
        shutil.copy2(source, target)
        changed[relative] = artifact(target)
    files = []
    for old_artifact in base["runner"]["files"]:
        path = Path(old_artifact["path"])
        new_path = root / path.relative_to(old) if path.is_relative_to(old) else path
        if new_path != version and sha(new_path) != old_artifact["sha256"]:
            raise RuntimeError(f"unchanged_runner_artifact_differs:{new_path.name}")
        files.append(artifact(new_path))
    for relative in ("files/lib/wine/x86_64-unix/winex11.so",):
        files.append(changed[relative])
    if len(files) != 32:
        raise RuntimeError("runner_roster")
    runner = {
        "id": RUNNER_ID,
        "version": RUNNER_VERSION,
        "proton": str(root / "proton"),
        "entry_point": base["runner"]["entry_point"],
        "files": files,
    }
    plan = {
        "schema": 1,
        "tree": tree_identity(root),
        "runner": runner,
        "changed_artifacts": changed,
        "base_runner_sha256": hashlib.sha256(json.dumps(base["runner"], sort_keys=True, separators=(",", ":")).encode()).hexdigest(),
    }
    write_json(args.build_root / "candidate-plan.private.json", plan)
    print(json.dumps({"candidate_tree": plan["tree"]["sha256"], "changed_artifacts": changed}, sort_keys=True))


def seal(args, base):
    plan = json.loads((args.build_root / "candidate-plan.private.json").read_text())
    if sha(args.patch) != PATCH_SHA or sha(args.source / "dlls/winex11.drv/mouse.c") != PATCHED_MOUSE_SHA \
            or sha(args.source / "dlls/winex11.drv/touch_message_flags.h") != TOUCH_HEADER_SHA:
        raise RuntimeError("patched_source_changed")
    if tree_identity(args.root) != plan["tree"]:
        raise RuntimeError("candidate_tree_changed")
    if not (args.build_root / "test_touch_flags").is_file():
        raise RuntimeError("mapping_test_absent")
    subprocess.run([str(args.build_root / "test_touch_flags")], check=True, timeout=10)
    smoke = args.build_root / "unlicensed-smoke.private"
    if smoke.exists() or smoke.is_symlink():
        raise RuntimeError("unlicensed_smoke_already_exists")
    for name in ("", "compatdata", "home", "client", "cache", "config", "data", "tmp", "runtime-var"):
        path = smoke / name
        path.mkdir(mode=0o700)
    environment = {
        "HOME": str(smoke / "home"), "USER": os.environ["USER"],
        "LOGNAME": os.environ["USER"], "PATH": "/usr/bin:/bin", "LANG": "C.UTF-8",
        "XDG_RUNTIME_DIR": f"/run/user/{os.getuid()}",
        "XDG_CACHE_HOME": str(smoke / "cache"),
        "XDG_CONFIG_HOME": str(smoke / "config"),
        "XDG_DATA_HOME": str(smoke / "data"),
        "TMPDIR": str(smoke / "tmp"),
        "STEAM_COMPAT_APP_ID": "0", "SteamAppId": "0", "SteamGameId": "0",
        "STEAM_COMPAT_CLIENT_INSTALL_PATH": str(smoke / "client"),
        "STEAM_COMPAT_DATA_PATH": str(smoke / "compatdata"),
        "PRESSURE_VESSEL_VARIABLE_DIR": str(smoke / "runtime-var"),
        "STEAM_ZENITY": "", "PROTON_LOG": "0",
    }
    session = subprocess.run(
        ["systemctl", "--user", "show-environment"], text=True,
        capture_output=True, check=True, timeout=10,
    )
    for line in session.stdout.splitlines():
        key, _, value = line.partition("=")
        if key in ("DISPLAY", "XAUTHORITY", "WAYLAND_DISPLAY", "DBUS_SESSION_BUS_ADDRESS"):
            environment[key] = value
    prefix = [base["runner"]["entry_point"], "--verb=run", "--", str(args.root / "proton")]
    prefix_root = smoke / "compatdata" / "pfx"
    cleanup_env = dict(environment, WINEPREFIX=str(prefix_root))
    try:
        initialized = subprocess.run(
            [*prefix, "getcompatpath", "/"], env=environment, text=True,
            capture_output=True, timeout=180, start_new_session=True,
        )
        if initialized.returncode != 0:
            raise RuntimeError(f"unlicensed_runner_initialize_failed:{initialized.returncode}")
        completed = subprocess.run(
            [*prefix, "runinprefix", "cmd.exe", "/d", "/c", "ver"],
            env=environment, text=True, capture_output=True, timeout=60,
            start_new_session=True,
        )
    finally:
        if prefix_root.is_dir():
            wineserver = str(args.root / "files/bin/wineserver")
            try:
                subprocess.run([wineserver, "-w"], env=cleanup_env,
                               capture_output=True, timeout=2, check=True)
            except subprocess.TimeoutExpired:
                subprocess.run([wineserver, "-k"], env=cleanup_env,
                               capture_output=True, timeout=15, check=True)
                subprocess.run([wineserver, "-w"], env=cleanup_env,
                               capture_output=True, timeout=15, check=True)
    if completed.returncode != 0 or "Microsoft Windows" not in completed.stdout:
        raise RuntimeError(f"unlicensed_runner_smoke_failed:{completed.returncode}")
    if tree_identity(args.root) != plan["tree"]:
        raise RuntimeError("candidate_tree_changed_during_smoke")
    receipt_path = args.build_root / "touch-build-receipt.private.json"
    receipt = {
        "schema": 1,
        "proton_distribution_source_commit": PROTON_SOURCE,
        "wine_commit": WINE_SOURCE,
        "wine_tree": WINE_TREE,
        "patch_sha256": PATCH_SHA,
        "patched_mouse_sha256": PATCHED_MOUSE_SHA,
        "touch_header_sha256": TOUCH_HEADER_SHA,
        "sdk_image_sha256": SDK_SHA,
        "configure": ["--enable-archs=i386,x86_64"],
        "install_prefix": str(args.stage),
        "configure_log": artifact(args.build_root / "configure.private.log"),
        "build_log": artifact(args.build_root / "build.private.log"),
        "install_log": artifact(args.build_root / "install.private.log"),
        "changed_artifacts": {key: value["sha256"] for key, value in plan["changed_artifacts"].items()},
        "candidate_tree_sha256": plan["tree"]["sha256"],
        "mapping_test_passed": True,
        "build_passed": True,
        "unlicensed_smoke_passed": True,
        "unlicensed_smoke_initialize_exit": initialized.returncode,
        "unlicensed_smoke_cmd_exit": completed.returncode,
        "unlicensed_smoke_cleanup_confirmed": True,
        "wow64_no_i386_unix_driver": True,
    }
    write_json(receipt_path, receipt)
    manifest = {
        "schema": 1,
        "kind": "x11_touch_release_reference_runner",
        "environment": base["id"],
        "base_runner_id": base["runner"]["id"],
        "base_runner_sha256": plan["base_runner_sha256"],
        "source": {"proton_distribution_source_commit": PROTON_SOURCE, "wine_commit": WINE_SOURCE, "wine_tree": WINE_TREE},
        "patch": artifact(args.patch),
        "build_receipt": artifact(receipt_path),
        "root": str(args.root),
        "tree": plan["tree"],
        "runner": plan["runner"],
        "changed_artifacts": plan["changed_artifacts"],
    }
    manifest_path = args.build_root / "touch-candidate-manifest.private.json"
    write_json(manifest_path, manifest)
    print(json.dumps({"manifest_sha256": sha(manifest_path), "candidate_tree": plan["tree"]["sha256"]}, sort_keys=True))


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", choices=("compose", "seal"))
    parser.add_argument("--environment-json", type=Path, required=True)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--stage", type=Path, required=True)
    parser.add_argument("--patch", type=Path, required=True)
    parser.add_argument("--build-root", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    arguments = parser.parse_args()
    if not all(path.is_absolute() for path in (arguments.environment_json, arguments.source,
            arguments.stage, arguments.patch, arguments.build_root, arguments.root)) \
            or arguments.source != arguments.build_root / "source" \
            or arguments.stage != arguments.build_root / "stage" \
            or arguments.patch != arguments.build_root / "wine-x11-touch-release.patch" \
            or arguments.root != arguments.build_root / f"candidate-{RUNNER_ID}":
        raise RuntimeError("private_build_paths")
    env = json.loads(arguments.environment_json.read_text())
    if arguments.mode == "compose":
        compose(arguments, env)
    else:
        seal(arguments, env)

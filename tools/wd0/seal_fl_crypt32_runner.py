#!/usr/bin/env python3
"""Seal the FL-only crypt32-order runner from the exact standard Proton closure.

The Wine change preserves the original signed-attribute order while verifying a
PKCS#7 signature. It does not disable or bypass signature verification.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess


RUNNER_ID = "proton-11.0-2c-fl-crypt32-order-v1"
BASE_ID = "proton-11.0-2c-25118279-slr4-4.0.20260805.254769"
VERSION = "1788504981 proton-11.0-2c-x86_64+fl-crypt32-order-v1; SLR 4.0.20260805.254769"
WINE_COMMIT = "dc26e61847081a1b5cb0733dc30feba6ee575482"
WINE_TREE = "da4b1eb3b7f209eb4a971d4b5929fcda08ff6b4e"
PROTON_COMMIT = "5b89db940e0ebe3a137a6009a3589232fe084c09"
SDK_SHA = "97526b794ce1a9bed5f891084462260b3a02399569f7438a3a57b5a253001db9"
UPSTREAM_PATCH_SHA = "e3cf1069975463c78506926b30f3f7f4d94169ef178d68a4eeb4b0a41f69e806"
SOURCE_SHA = {
    "dlls/crypt32/crypt32_private.h": "4161501bbfea841c08bcdcf7ac8012da6544f1b1c527efe79c2b1165dfecbd19",
    "dlls/crypt32/encode.c": "b736b05d134e236260a1a9e58b7e1a05c7d5006d6ecf5c3eae2353d1d832a77c",
    "dlls/crypt32/msg.c": "89c24b49dc458cae7610b195ec36f0f2796a462a04284975368227a2f83572ca",
    "dlls/crypt32/tests/msg.c": "ff5ee9df8ebd3e22a814cc2b549197d605fcb23079fac70f477ac3d8f5be05a9",
}
AUTOGEN_SHA = {
    "include/wine/server_protocol.h": "acaf87f5988b8c1b6cab07f28889aeb577770fce92424ce355457aba9a500a27",
    "server/request_handlers.h": "aa7ff287ee226add950af996443bba9ab2ad46e9aca25591e5de880d230d351d",
    "server/request_trace.h": "d2e987104bf8d34fb23819cd7c6bada2d202f9e1306227c3524424b5eafed2cc",
}
CHANGED = (
    "version",
    "files/lib/wine/i386-windows/crypt32.dll",
    "files/lib/wine/x86_64-windows/crypt32.dll",
)


def sha(path):
    h = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def artifact(path):
    return {"path": str(path), "sha256": sha(path)}


def rows(root):
    result, pending = {}, [root]
    while pending:
        for path in pending.pop().iterdir():
            md = path.lstat()
            rel = path.relative_to(root).as_posix()
            mode = md.st_mode & 0o7777
            if stat.S_ISDIR(md.st_mode):
                pending.append(path)
                row = ["d", rel, mode]
            elif stat.S_ISREG(md.st_mode):
                row = ["f", rel, mode, md.st_size, sha(path)]
            elif stat.S_ISLNK(md.st_mode):
                if not path.resolve(strict=True).is_relative_to(root):
                    raise RuntimeError("outside_runner_link:" + rel)
                row = ["l", rel, mode, os.readlink(path)]
            else:
                raise RuntimeError("unsupported_runner_type:" + rel)
            result[rel] = row
    return result


def tree_identity(value):
    h = hashlib.sha256()
    for rel in sorted(value):
        h.update(json.dumps(value[rel], separators=(",", ":"), ensure_ascii=False).encode())
        h.update(b"\n")
    return {"schema": 1, "entries": len(value),
            "regular_bytes": sum(row[3] for row in value.values() if row[0] == "f"),
            "sha256": h.hexdigest()}


def write_private(path, value):
    data = (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o400)
    with os.fdopen(fd, "wb") as output:
        output.write(data)
        output.flush()
        os.fsync(output.fileno())


def source_check(root):
    sdk = subprocess.check_output(
        ["podman", "image", "inspect",
         "registry.gitlab.steamos.cloud/proton/steamrt4/sdk/x86_64:4.0.20260331.220802-0",
         "--format", "{{json .RepoDigests}}"], text=True, timeout=10)
    if not any(value.endswith("@sha256:" + SDK_SHA) for value in json.loads(sdk)):
        raise RuntimeError("offline_sdk_changed")
    source = root / "source"
    for ref, expected in (("HEAD", WINE_COMMIT), ("HEAD^{tree}", WINE_TREE)):
        actual = subprocess.check_output(["git", "-C", str(source), "rev-parse", ref], text=True).strip()
        if actual != expected:
            raise RuntimeError("wine_source_changed")
    if sha(root / "upstream-11824.patch") != UPSTREAM_PATCH_SHA:
        raise RuntimeError("upstream_patch_changed")
    for rel, expected in SOURCE_SHA.items():
        if sha(source / rel) != expected:
            raise RuntimeError("patched_source_changed:" + rel)
    for rel, expected in AUTOGEN_SHA.items():
        if sha(source / rel) != expected:
            raise RuntimeError("generated_source_changed:" + rel)
    tracked_delta = subprocess.check_output(
        ["git", "-C", str(source), "diff", "--name-only"], text=True).splitlines()
    if set(tracked_delta) != set(SOURCE_SHA) | set(AUTOGEN_SHA):
        raise RuntimeError("wine_source_delta_changed")
    status = subprocess.run(["git", "-C", str(source), "diff", "--check"], check=True)
    del status


def compose(args):
    source_check(args.build_root)
    environment = json.loads(args.environment_json.read_text())
    before = environment["runner"]
    if environment["revision"] != 1 or before["id"] != BASE_ID or before.get("policy") is not None:
        raise RuntimeError("standard_fl_predecessor_required")
    for item in before["files"]:
        if sha(Path(item["path"])) != item["sha256"]:
            raise RuntimeError("base_runner_artifact_changed")
    base = Path(before["proton"]).parent
    target = args.root
    if not target.exists():
        shutil.copytree(base, target, symlinks=True)
    if target.is_symlink() or not target.is_dir():
        raise RuntimeError("candidate_runner_type")
    target.chmod(0o700)
    original = rows(base)
    cloned = rows(target)
    built = args.build_root / "obj/dlls/crypt32"
    if cloned == original:
        version = target / "version"
        version.chmod(version.stat().st_mode | stat.S_IWUSR)
        version.write_text(VERSION.split(";", 1)[0] + "\n")
        for arch in ("i386", "x86_64"):
            rel = f"files/lib/wine/{arch}-windows/crypt32.dll"
            generated = built / f"{arch}-windows/crypt32.dll"
            if not generated.is_file() or generated.is_symlink():
                raise RuntimeError("built_pe_crypt32_missing:" + arch)
            destination = target / rel
            destination.chmod(destination.stat().st_mode | stat.S_IWUSR)
            shutil.copy2(generated, destination)
    else:
        if (target / "version").read_text() != VERSION.split(";", 1)[0] + "\n":
            raise RuntimeError("candidate_prestate_not_base")
        for arch in ("i386", "x86_64"):
            rel = f"files/lib/wine/{arch}-windows/crypt32.dll"
            if sha(target / rel) != sha(built / f"{arch}-windows/crypt32.dll"):
                raise RuntimeError("candidate_prestate_not_base")
    changed = rows(target)
    if set(changed) != set(original) or {name for name in original if original[name] != changed[name]} != set(CHANGED):
        raise RuntimeError("runner_changed_artifact_roster")
    runner_files = []
    for item in before["files"]:
        old_path = Path(item["path"])
        new_path = target / old_path.relative_to(base) if old_path.is_relative_to(base) else old_path
        current = artifact(new_path)
        if not old_path.is_relative_to(base) or old_path.relative_to(base).as_posix() not in CHANGED:
            if current["sha256"] != item["sha256"]:
                raise RuntimeError("unchanged_runner_artifact_changed")
        runner_files.append(current)
    for rel in CHANGED[1:]:
        path = target / rel
        if not any(row["path"] == str(path) for row in runner_files):
            runner_files.append(artifact(path))
    runner = {"id": RUNNER_ID, "version": VERSION, "proton": str(target / "proton"),
              "entry_point": before["entry_point"], "files": runner_files}
    retained_patch = target.parent / "upstream-11824.patch"
    if not retained_patch.exists():
        shutil.copy2(args.build_root / "upstream-11824.patch", retained_patch)
    if sha(retained_patch) != UPSTREAM_PATCH_SHA:
        raise RuntimeError("retained_upstream_patch_changed")
    manifest = {"schema": 1, "kind": "fl_crypt32_order_reference_runner",
                "workspace": environment["id"], "base_runner": before,
                "source": {"proton": PROTON_COMMIT, "wine": WINE_COMMIT, "wine_tree": WINE_TREE,
                           "upstream_merge_request": "wine/wine!11824",
                           "upstream_patch": artifact(retained_patch),
                           "patched_files": SOURCE_SHA, "sdk_image_sha256": SDK_SHA},
                "runner": runner, "root": str(target), "tree": tree_identity(changed),
                "changed_artifacts": {rel: artifact(target / rel) for rel in CHANGED}}
    write_private(target.parent / "fl-runner-manifest.private.json", manifest)
    print(json.dumps({"tree": manifest["tree"], "runner": RUNNER_ID,
                      "changed_artifacts": manifest["changed_artifacts"]}, sort_keys=True))


def verify(args):
    manifest_path = args.root.parent / "fl-runner-manifest.private.json"
    manifest = json.loads(manifest_path.read_text())
    source_check(args.build_root)
    if manifest["root"] != str(args.root) or manifest["kind"] != "fl_crypt32_order_reference_runner":
        raise RuntimeError("runner_manifest_identity")
    if tree_identity(rows(args.root)) != manifest["tree"]:
        raise RuntimeError("runner_tree_changed")
    for item in manifest["runner"]["files"]:
        if sha(Path(item["path"])) != item["sha256"]:
            raise RuntimeError("runner_artifact_changed")
    print(json.dumps({"manifest_sha256": sha(manifest_path), "tree": manifest["tree"]["sha256"]}))


def smoke(args):
    verify(args)
    manifest = json.loads((args.root.parent / "fl-runner-manifest.private.json").read_text())
    workspace = args.root.parent / "unlicensed-fl-smoke.private"
    if workspace.exists() or workspace.is_symlink():
        raise RuntimeError("smoke_root_exists")
    for name in ("", "compatdata", "home", "client", "cache", "config", "data", "tmp", "runtime-var"):
        (workspace / name).mkdir(mode=0o700)
    user = os.environ["USER"]
    env = {"HOME": str(workspace / "home"), "USER": user, "LOGNAME": user,
           "PATH": "/usr/bin:/bin", "LANG": "C.UTF-8",
           "XDG_RUNTIME_DIR": f"/run/user/{os.getuid()}",
           "XDG_CACHE_HOME": str(workspace / "cache"),
           "XDG_CONFIG_HOME": str(workspace / "config"),
           "XDG_DATA_HOME": str(workspace / "data"), "TMPDIR": str(workspace / "tmp"),
           "STEAM_COMPAT_APP_ID": "0", "SteamAppId": "0", "SteamGameId": "0",
           "STEAM_COMPAT_CLIENT_INSTALL_PATH": str(workspace / "client"),
           "STEAM_COMPAT_DATA_PATH": str(workspace / "compatdata"),
           "PRESSURE_VESSEL_VARIABLE_DIR": str(workspace / "runtime-var"),
           "STEAM_ZENITY": "", "PROTON_LOG": "0"}
    session = subprocess.run(["systemctl", "--user", "show-environment"], check=True,
                             text=True, capture_output=True, timeout=10)
    for line in session.stdout.splitlines():
        key, _, value = line.partition("=")
        if key in ("DISPLAY", "XAUTHORITY", "WAYLAND_DISPLAY", "DBUS_SESSION_BUS_ADDRESS"):
            env[key] = value
    command = [manifest["runner"]["entry_point"], "--verb=run", "--",
               manifest["runner"]["proton"]]
    prefix = workspace / "compatdata/pfx"
    try:
        initialized = subprocess.run([*command, "getcompatpath", "/"], env=env,
                                     text=True, capture_output=True, timeout=180,
                                     start_new_session=True)
        if initialized.returncode:
            raise RuntimeError("unlicensed_runner_initialize_failed")
        version = subprocess.run([*command, "runinprefix", "cmd.exe", "/d", "/c", "ver"],
                                 env=env, text=True, capture_output=True, timeout=60,
                                 start_new_session=True)
        test = subprocess.run([*command, "runinprefix",
                               str(args.build_root / "obj/dlls/crypt32/tests/x86_64-windows/crypt32_test.exe"),
                               "msg"], env=env, text=True, capture_output=True, timeout=180,
                              start_new_session=True)
    finally:
        if prefix.is_dir():
            cleanup_env = dict(env, WINEPREFIX=str(prefix))
            wineserver = str(args.root / "files/bin/wineserver")
            try:
                subprocess.run([wineserver, "-w"], env=cleanup_env, check=True,
                               capture_output=True, timeout=5)
            except subprocess.TimeoutExpired:
                subprocess.run([wineserver, "-k"], env=cleanup_env, check=True,
                               capture_output=True, timeout=15)
                subprocess.run([wineserver, "-w"], env=cleanup_env, check=True,
                               capture_output=True, timeout=15)
    receipt = {"schema": 1, "kind": "fl_crypt32_order_unlicensed_smoke",
               "runner_tree": manifest["tree"]["sha256"],
               "initialize_exit": initialized.returncode, "cmd_exit": version.returncode,
               "crypt32_msg_exit": test.returncode,
               "crypt32_msg_stdout_tail": test.stdout[-2000:],
               "crypt32_msg_stderr_tail": test.stderr[-2000:],
               "cleanup_confirmed": True}
    write_private(args.root.parent / "fl-runner-smoke.private.json", receipt)
    if version.returncode or "Microsoft Windows" not in version.stdout or test.returncode:
        raise RuntimeError("unlicensed_runner_or_crypt32_test_failed")
    verify(args)


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("mode", choices=("compose", "verify", "smoke"))
    p.add_argument("--environment-json", type=Path, required=True)
    p.add_argument("--build-root", type=Path, required=True)
    p.add_argument("--root", type=Path, required=True)
    args = p.parse_args()
    if not args.environment_json.is_absolute() or not args.build_root.is_absolute() or not args.root.is_absolute():
        raise RuntimeError("absolute_private_paths_required")
    {"compose": compose, "verify": verify, "smoke": smoke}[args.mode](args)

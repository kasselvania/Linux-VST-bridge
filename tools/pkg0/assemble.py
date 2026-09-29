#!/usr/bin/env python3
"""Stage only declared binary files for a PKG0 Arch package build.

This is a builder-side tool. The input specification may name private paths;
only destination names, digests and component identities enter the package.
"""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import secrets
import shutil
import stat
import tarfile
import zipfile

PACKAGE = "linux-vst-bridge-beta"
PKGREL = 1
DEPENDENCIES = ("glibc", "gcc-libs", "python", "systemd", "libx11", "libxcb",
                "libxkbcommon", "libglvnd", "pipewire", "xdg-desktop-portal")
REQUIRED = {
    "usr/bin/linux-vst-bridge": "manager",
    "usr/bin/linux-audio-compatibility-manager": "frontend",
    "usr/lib/linux-vst-bridge/supervisor/session.pyc": "supervisor",
    "usr/lib/linux-vst-bridge/supervisor/ownership.pyc": "ownership",
    "usr/lib/linux-vst-bridge/host/bridge-host.exe": "windows_host",
    "usr/lib/linux-vst-bridge/host/source-manifest.json": "host_source",
    "usr/share/doc/linux-vst-bridge-beta/START_HERE.html": "guide",
    "usr/share/doc/linux-vst-bridge-beta/THIRD_PARTY_NOTICES.txt": "notices",
    "usr/share/doc/linux-vst-bridge-beta/SBOM.spdx.json": "sbom",
    "usr/share/doc/linux-vst-bridge-beta/COMPLIANCE_MANIFEST.json": "compliance",
}
ADOPTION_MANIFEST = "usr/share/linux-vst-bridge/pkg0-manifest.json"
KIT_DESTINATION = "usr/lib/linux-vst-bridge/preparation/preparation-kit.zip"
ADOPTED = {
    "usr/bin/linux-vst-bridge": "linux-vst-bridge",
    "usr/bin/linux-audio-compatibility-manager": "linux-audio-compatibility-manager",
    "usr/lib/linux-vst-bridge/supervisor/session.pyc": "session.pyc",
    "usr/lib/linux-vst-bridge/supervisor/ownership.pyc": "ownership.pyc",
    "usr/lib/linux-vst-bridge/host/bridge-host.exe": "host.exe",
    "usr/lib/linux-vst-bridge/host/source-manifest.json": "host-source-manifest.json",
}
ADOPTED_WITH_KIT = {**ADOPTED, KIT_DESTINATION: "preparation-kit.zip"}
SOURCE_SUFFIXES = {".rs", ".c", ".cc", ".cpp", ".h", ".hpp", ".py"}
SECRET_MARKERS = (b"-----BEGIN PRIVATE KEY-----", b"-----BEGIN OPENSSH PRIVATE KEY-----",
                  b"github_pat_", b"ghp_")


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def sha(data):
    return hashlib.sha256(data).hexdigest()


def path_ok(name):
    p = PurePosixPath(name)
    parts = p.parts
    if str(p) != name or p.is_absolute() or not parts or parts[0] != "usr" or any(
        x in ("", ".", "..", ".git", "target", "build", "__pycache__") for x in parts
    ):
        raise ValueError("package destination outside controlled usr tree")
    if not (name.startswith("usr/bin/") or name.startswith("usr/lib/linux-vst-bridge/")
            or name.startswith("usr/share/doc/linux-vst-bridge-beta/")):
        raise ValueError("package destination not admitted")
    if any(x in name.lower() for x in ("credential", "activation", "private-key", "installer.exe")):
        raise ValueError("sensitive destination name")


def source_file(raw):
    p = Path(raw)
    if not p.is_absolute() or p.is_symlink():
        raise ValueError("builder input must be an absolute regular file")
    md = p.stat()
    if not stat.S_ISREG(md.st_mode) or md.st_uid != os.getuid() or md.st_size > 2_000_000_000:
        raise ValueError("builder input owner, type or size")
    return p


def file_bytes(item):
    p = source_file(item["source"])
    before = p.stat()
    if item["kind"] == "preparation_kit" and before.st_size > 256 * 1024 * 1024:
        raise ValueError("preparation kit extent")
    data = p.read_bytes()
    after = p.stat()
    stamp = lambda m: (m.st_dev, m.st_ino, m.st_size, m.st_mtime_ns, m.st_ctime_ns)
    if stamp(before) != stamp(after) or sha(data) != item["sha256"]:
        raise ValueError("builder input changed or digest differs")
    if any(marker in data for marker in SECRET_MARKERS):
        raise ValueError("secret-like bytes in package input")
    return data


def verify_kit(data, source_head, host_sha256, source_sha256):
    if len(data) > 256 * 1024 * 1024:
        raise ValueError("preparation kit extent")
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        entries = archive.infolist()
        names = [entry.filename for entry in entries]
        if len(names) > 512 or len(names) != len(set(names)) or "recipe.json" not in names:
            raise ValueError("preparation kit roster")
        total = 0
        fixed = {"recipe.json", "CMakeLists.txt", "native-vst3-proxy/CMakeLists.txt",
                 "cmake/HP0ModernGcc.cmake",
                 "cmake/HP0Vst3SdkLock.cmake", "libap2_backend.a",
                 "runtime/host.exe", "runtime/host-source-manifest.json",
                 "tools/mf3/native_builder.py", "tools/ap8_descriptor.py"}
        for entry in entries:
            name = entry.filename
            path = PurePosixPath(name)
            entry_type = stat.S_IFMT(entry.external_attr >> 16)
            source_path = (name.startswith("native-vst3-proxy/")
                           and path.suffix in {".rs", ".h", ".cpp", ".c", ".toml", ".lock", ".py"})
            state_path = name.startswith("vst-state/") and path.suffix in {".h", ".cpp"}
            if (entry.is_dir() or path.is_absolute() or str(path) != name
                    or any(part in ("", ".", "..") for part in path.parts)
                    or name not in fixed and not source_path and not state_path
                    or entry_type not in (0, stat.S_IFREG)
                    or entry.file_size > 128 * 1024 * 1024):
                raise ValueError("preparation kit entry")
            total += entry.file_size
            if total > 512 * 1024 * 1024:
                raise ValueError("preparation kit extent")
        if archive.getinfo("recipe.json").file_size > 65536:
            raise ValueError("preparation kit recipe extent")
        recipe = json.loads(archive.read("recipe.json"))
        if (set(recipe) != {"schema", "source_commit", "sdk", "sdk_runtime", "files"}
                or recipe["schema"] != 2 or recipe["source_commit"] != source_head
                or not isinstance(recipe["files"], dict)
                or set(recipe["files"]) != set(names) - {"recipe.json"}
                or not isinstance(recipe["sdk"], str)
                or not isinstance(recipe["sdk_runtime"], str)):
            raise ValueError("preparation kit recipe")
        required = {"libap2_backend.a", "runtime/host.exe",
                    "runtime/host-source-manifest.json", "tools/mf3/native_builder.py",
                    "tools/ap8_descriptor.py", "CMakeLists.txt"}
        if not required <= set(recipe["files"]):
            raise ValueError("preparation kit required files")
        for name, expected in recipe["files"].items():
            source = archive.read(name)
            if (not re.fullmatch(r"[0-9a-f]{64}", expected)
                    or sha(source) != expected
                    or any(marker in source for marker in SECRET_MARKERS)):
                raise ValueError("preparation kit file digest")
        if (recipe["files"]["runtime/host.exe"] != host_sha256
                or recipe["files"]["runtime/host-source-manifest.json"] != source_sha256):
            raise ValueError("preparation kit host pair")


def validate(spec):
    base = {"schema", "version", "source_head", "source_tree", "operator_schema",
            "external_runtime", "files"}
    if set(spec) != base or spec["schema"] not in (1, 2):
        raise ValueError("PKG0 input schema")
    if spec["operator_schema"] != 12:
        raise ValueError("paired operator schema differs")
    external = spec["external_runtime"]
    if (set(external) != {"id", "manifest_sha256"}
            or not re.fullmatch(r"[A-Za-z0-9_-]{1,128}", external["id"])
            or not re.fullmatch(r"[0-9a-f]{64}", external["manifest_sha256"])):
        raise ValueError("external runtime identity")
    if not re.fullmatch(r"[0-9][A-Za-z0-9.]*", spec["version"]):
        raise ValueError("Arch package version syntax")
    if any(not re.fullmatch(r"[0-9a-f]{40}", spec[k]) for k in ("source_head", "source_tree")):
        raise ValueError("source identity syntax")
    if not isinstance(spec["files"], list) or len(spec["files"]) > 100000:
        raise ValueError("package file count")
    seen = {}
    for item in spec["files"]:
        expected = {"destination", "source", "sha256", "kind", "component", "mode"}
        if item.get("kind") in ("manager", "frontend"):
            expected |= {"build_head", "build_tree"}
            if ((item.get("build_head"), item.get("build_tree"))
                    != (spec["source_head"], spec["source_tree"])):
                raise ValueError("paired manager/frontend build generation differs")
        if set(item) != expected:
            raise ValueError("file input schema")
        name = item["destination"]
        path_ok(name)
        if name == ADOPTION_MANIFEST or name in seen:
            raise ValueError("duplicate or reserved package destination")
        if not re.fullmatch(r"[0-9a-f]{64}", item["sha256"]):
            raise ValueError("file digest syntax")
        if item["mode"] not in ("0444", "0555") or item["kind"] not in (
            "manager", "frontend", "supervisor", "ownership", "windows_host",
            "host_source", "proxy", "fixture", "preparation_kit",
            "fixture_resource",
            "profile", "guide", "notices", "sbom", "compliance", "license"
        ):
            raise ValueError("file mode or role")
        if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9.+_-]{0,80}", item["component"]):
            raise ValueError("component identity")
        if name in REQUIRED and REQUIRED[name] != item["kind"]:
            raise ValueError("required package role")
        if name not in REQUIRED:
            prefix = {
                "proxy": "usr/lib/linux-vst-bridge/proxy/",
                "preparation_kit": "usr/lib/linux-vst-bridge/preparation/",
                "fixture": "usr/lib/linux-vst-bridge/self-test/",
                "fixture_resource": "usr/lib/linux-vst-bridge/self-test/",
                "profile": "usr/lib/linux-vst-bridge/profiles/",
                "license": "usr/share/doc/linux-vst-bridge-beta/licenses/",
            }.get(item["kind"])
            if prefix is None or not name.startswith(prefix):
                raise ValueError("file role and destination disagree")
        if item["kind"] == "preparation_kit" and (name != KIT_DESTINATION or item["mode"] != "0444"):
            raise ValueError("preparation kit destination or mode")
        if item["kind"] != "license" and PurePosixPath(name).suffix in SOURCE_SUFFIXES:
            raise ValueError("proprietary source in binary package")
        seen[name] = item
    if any(name not in seen for name in REQUIRED):
        raise ValueError("required binary/document missing")
    if (KIT_DESTINATION in seen) != (spec["schema"] == 2):
        raise ValueError("preparation kit required by package schema")
    if not any(x["kind"] == "proxy" for x in spec["files"]):
        raise ValueError("native proxy absent")
    if not any(x["kind"] == "fixture" for x in spec["files"]):
        raise ValueError("self-test fixture absent")
    if not any(x["kind"] == "profile" for x in spec["files"]):
        raise ValueError("supported profile absent")
    if not any(x["kind"] == "license" for x in spec["files"]):
        raise ValueError("third-party license text absent")
    return seen


def _build(spec, output, epoch):
    files = validate(spec)
    if output.exists():
        raise ValueError("output already exists")
    output.mkdir(parents=True, mode=0o700)
    payload = output / "payload.tar"
    roster = []
    with tarfile.open(payload, "w", format=tarfile.PAX_FORMAT) as archive:
        host_sha256 = files["usr/lib/linux-vst-bridge/host/bridge-host.exe"]["sha256"]
        source_sha256 = files["usr/lib/linux-vst-bridge/host/source-manifest.json"]["sha256"]
        for name, item in sorted(files.items()):
            data = file_bytes(item)
            if item["kind"] == "preparation_kit":
                verify_kit(data, spec["source_head"], host_sha256, source_sha256)
            if item["kind"] in ("manager", "frontend", "proxy") and not data.startswith(b"\x7fELF"):
                raise ValueError("Linux executable format")
            if item["kind"] in ("windows_host", "fixture") and not data.startswith(b"MZ"):
                raise ValueError("Windows executable format")
            if item["kind"] in ("supervisor", "ownership") and not name.endswith(".pyc"):
                raise ValueError("supervisor bytecode required")
            add_bytes(archive, name, data, int(item["mode"], 8), epoch)
            roster.append({"destination": name, "sha256": sha(data), "size": len(data),
                           "mode": item["mode"], "kind": item["kind"], "component": item["component"]})
        adopted_names = ADOPTED_WITH_KIT if spec["schema"] == 2 else ADOPTED
        adopted = [{"name": adopted_names[name], "sha256": files[name]["sha256"],
                    "size": len(file_bytes(files[name]))} for name in adopted_names]
        adoption = canonical({"schema": spec["schema"], "package": PACKAGE, "version": spec["version"],
                              "pkgrel": PKGREL,
                              "source_head": spec["source_head"],
                              "source_tree": spec["source_tree"],
                              "operator_schema": spec["operator_schema"],
                              "files": adopted, "external_runtime": spec["external_runtime"]})
        add_bytes(archive, ADOPTION_MANIFEST, adoption, 0o444, epoch)
        roster.append({"destination": ADOPTION_MANIFEST, "sha256": sha(adoption),
                       "size": len(adoption), "mode": "0444",
                       "kind": "adoption_manifest", "component": PACKAGE})
    manifest = {"schema": 1, "package": PACKAGE, "version": spec["version"],
                "pkgrel": PKGREL,
                "source_head": spec["source_head"], "source_tree": spec["source_tree"],
                "payload_sha256": sha(payload.read_bytes()), "files": roster}
    (output / "RELEASE_MANIFEST.json").write_bytes(canonical(manifest))
    pkgbuild = f'''pkgname={PACKAGE}
pkgver={spec["version"]}
pkgrel={PKGREL}
pkgdesc="Private Linux VST Bridge beta for native Linux DAWs"
arch=('x86_64')
url='https://github.com/kasselvania/Linux-VST-bridge'
license=('LicenseRef-Proprietary')
depends=({' '.join(repr(dependency) for dependency in DEPENDENCIES)})
options=('!strip' '!debug' '!lto')
source=('payload.tar')
sha256sums=('{manifest["payload_sha256"]}')
package() {{
  bsdtar -xf "$srcdir/payload.tar" -C "$pkgdir"
}}
'''
    (output / "PKGBUILD").write_text(pkgbuild)
    return manifest


def build(spec, output, epoch):
    output = Path(output)
    if output.exists():
        raise ValueError("output already exists")
    temporary = output.with_name(output.name + ".partial-" + secrets.token_hex(8))
    try:
        manifest = _build(spec, temporary, epoch)
        temporary.rename(output)
        return manifest
    finally:
        if temporary.exists():
            shutil.rmtree(temporary)


def add_bytes(archive, name, data, mode, epoch):
    import io
    info = tarfile.TarInfo(name)
    info.size = len(data)
    info.mode = mode
    info.uid = info.gid = 0
    info.mtime = epoch
    archive.addfile(info, io.BytesIO(data))


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--spec", type=Path, required=True)
    p.add_argument("--output", type=Path, required=True)
    p.add_argument("--source", type=Path, required=True)
    a = p.parse_args()
    source = a.source.resolve(strict=True)
    import subprocess
    head = subprocess.check_output(["git", "-C", str(source), "rev-parse", "HEAD"], text=True).strip()
    tree = subprocess.check_output(["git", "-C", str(source), "rev-parse", "HEAD^{tree}"], text=True).strip()
    if subprocess.check_output(["git", "-C", str(source), "status", "--porcelain"]):
        raise ValueError("commit package source before assembly")
    spec = json.loads(a.spec.read_bytes())
    if (spec.get("source_head"), spec.get("source_tree")) != (head, tree):
        raise ValueError("package source head/tree differs")
    epoch = int(subprocess.check_output(["git", "-C", str(source), "show", "-s", "--format=%ct", "HEAD"]).strip())
    print(json.dumps(build(spec, a.output, epoch), sort_keys=True))


if __name__ == "__main__":
    main()

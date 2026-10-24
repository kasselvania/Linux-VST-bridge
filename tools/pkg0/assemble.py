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
import signal
import stat
import subprocess
import tarfile
import tempfile
import zipfile

PACKAGE = "linux-vst-bridge-beta"
PKGREL = 1
DEPENDENCIES = ("glibc", "gcc-libs", "systemd", "curl", "libx11", "libxcb",
                "libxkbcommon", "libxkbcommon-x11", "libxcursor", "libxi",
                "libglvnd", "pipewire", "xdg-desktop-portal")
SUPERVISOR_PATHS = (
    "usr/lib/linux-vst-bridge/supervisor/session.pyc",
    "usr/lib/linux-vst-bridge/supervisor/ownership.pyc",
)
SOURCE_SUPERVISOR_PATHS = tuple(path[:-1] for path in SUPERVISOR_PATHS)
# Every range source supervisors have declared, the current one last; it must
# match bridge-manager/runtime/bootstrap.py. A newer checkout still verifies a
# package built while an earlier range was current.
SOURCE_PYTHON_RANGES = ("3.13-3.14", "3.13-3.15")
SOURCE_PYTHON_RANGE = SOURCE_PYTHON_RANGES[-1]


def source_python_bounds(python_range):
    """The floor and the first excluded minor: ("3.13", "3.16") for "3.13-3.15"."""
    if python_range not in SOURCE_PYTHON_RANGES:
        raise ValueError("unsupported source Python range")
    low, high = python_range.split("-")
    major, minor = (int(part) for part in high.split("."))
    return low, f"{major}.{minor + 1}"


def source_supervisor(paths):
    present = set(paths) & set((*SUPERVISOR_PATHS, *SOURCE_SUPERVISOR_PATHS))
    if present == set(SOURCE_SUPERVISOR_PATHS):
        return True
    if present == set(SUPERVISOR_PATHS):
        return False
    raise ValueError("supervisor source/bytecode pair missing or mixed")


# Exact CPython bytecode magics for the declared Debian 13 and Ubuntu 26.04
# build targets. Arch intake must declare the same interpreter ABI as its
# packaged supervisors; a rolling host cannot infer that from plain `python`.
PYTHON_MINOR_BY_MAGIC = {
    bytes.fromhex("f30d0d0a"): "3.13",
    bytes.fromhex("2b0e0d0a"): "3.14",
}


def supervisor_python_minor(headers):
    if set(headers) != set(SUPERVISOR_PATHS):
        raise ValueError("supervisor Python ABI missing")
    magics = set()
    for header in headers.values():
        if len(header) != 16 or int.from_bytes(header[4:8], "little") not in (0, 1, 3):
            raise ValueError("supervisor Python bytecode header")
        magics.add(header[:4])
    if len(magics) != 1 or next(iter(magics)) not in PYTHON_MINOR_BY_MAGIC:
        raise ValueError("supervisor Python ABI mismatch or unsupported")
    return PYTHON_MINOR_BY_MAGIC[next(iter(magics))]


def package_dependencies(python_minor):
    if python_minor in SOURCE_PYTHON_RANGES:
        low, ceiling = source_python_bounds(python_minor)
        return (*DEPENDENCIES, f"python>={low}", f"python<{ceiling}")
    if python_minor not in PYTHON_MINOR_BY_MAGIC.values():
        raise ValueError("unsupported package Python ABI")
    major, minor = (int(part) for part in python_minor.split("."))
    return (*DEPENDENCIES, f"python>={major}.{minor}", f"python<{major}.{minor + 1}")


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
SOURCE_REQUIRED = {name[:-1] if name in SUPERVISOR_PATHS else name: kind
                   for name, kind in REQUIRED.items()}


ADOPTION_MANIFEST = "usr/share/linux-vst-bridge/pkg0-manifest.json"
SYSTEM_DESKTOP = "usr/share/applications/linux-vst-bridge-setup.desktop"
SYSTEM_DESKTOP_BYTES = b"""[Desktop Entry]
Type=Application
Name=Linux VST Bridge Setup and Updates
Comment=Set up or update the managed audio application
Exec=/usr/bin/linux-audio-compatibility-manager
Icon=audio-card
Terminal=false
Categories=AudioVideo;Audio;
Keywords=Linux Audio Compatibility Manager;Bridge;Update;
StartupNotify=true
"""
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
SOURCE_ADOPTED = {name[:-1] if name in SUPERVISOR_PATHS else name:
                  value[:-1] if name in SUPERVISOR_PATHS else value
                  for name, value in ADOPTED_WITH_KIT.items()}
SOURCE_SUFFIXES = {".rs", ".c", ".cc", ".cpp", ".h", ".hpp", ".py"}
EXECUTABLE_KINDS = {"manager", "frontend", "proxy"}
OPTIONAL_PREFIXES = {
    "proxy": "usr/lib/linux-vst-bridge/proxy/",
    "preparation_kit": "usr/lib/linux-vst-bridge/preparation/",
    "fixture": "usr/lib/linux-vst-bridge/self-test/",
    "fixture_resource": "usr/lib/linux-vst-bridge/self-test/",
    "profile": "usr/lib/linux-vst-bridge/profiles/",
    "license": "usr/share/doc/linux-vst-bridge-beta/licenses/",
}
SECRET_MARKERS = (b"-----BEGIN PRIVATE KEY-----", b"-----BEGIN OPENSSH PRIVATE KEY-----",
                  b"github_pat_", b"ghp_")
KIT_SDK = "3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96"
KIT_SDK_RUNTIME = "b90ed309cc1d505dea48b6a2121c5dcfac22868120eee643b0596d31f96b9bb8"
KIT_RUST_TOOLCHAIN = "1.95.0"
KIT_SOURCE_ARGS = ("CMakeLists.txt", "cmake/HP0Vst3SdkLock.cmake",
                   "cmake/HP0ModernGcc.cmake", "native-vst3-proxy",
                   "vst-state", "tools/mf3/native_builder.py",
                   "tools/ap8_descriptor.py")
KIT_GENERATED = {"libap2_backend.a", "runtime/host.exe",
                 "runtime/host-source-manifest.json"}
LAUNCH_SOURCES = {"tools/is2/launch.cpp", "tools/is2/nad1_service.h",
                  "tools/is2/nad1_stop.h", "tools/is2/native_access_callback.h"}
LAUNCH_PRODUCER = "tools/naui2/package.py"
LAUNCH_FLAGS = ("-target", "x86_64-windows-gnu", "-std=c++20", "-O2", "-municode",
                "-DUNICODE", "-D_UNICODE", "-lbcrypt", "-ladvapi32")
LAUNCH_MEMBERS = {"runtime/installer-launch.exe", "runtime/installer-launch-source.json",
                  "licenses/installer-launch.txt"}


def library_locations_capability(source):
    """The signed adoption record binds the declared plan to exact helper bytes."""
    import ast
    module = ast.parse(source)
    values = [node.value.value for node in module.body
              if isinstance(node, ast.Assign) and isinstance(node.value, ast.Constant)
              and any(isinstance(target, ast.Name) and target.id == "LIBRARY_LOCATIONS_PLAN_SCHEMA"
                      for target in node.targets)]
    if not values:
        return None
    if values != [1] or type(values[0]) is not int:
        raise ValueError("library location supervisor capability")
    return {"schema": 1, "supervisor_sha256": sha(source)}


def parse_operator_schema(source):
    matches = re.findall(r"^pub const OPERATOR_SCHEMA: u32 = ([0-9]+);$",
                         source, re.MULTILINE)
    if len(matches) != 1 or not 1 <= int(matches[0]) <= 0xffffffff:
        raise ValueError("declared operator schema missing or invalid")
    return int(matches[0])


# Where a source tree declares its operator model. Trees from before the model
# crate (crates/model) declare it in the manager.
OPERATOR_MODEL_SOURCES = ("crates/model/src/operator_model.rs",
                          "bridge-manager/src/operator_model.rs")


def declared_operator_schema(source_root=None):
    root = Path(source_root) if source_root is not None else Path(__file__).resolve().parents[2]
    model = next((root / name for name in OPERATOR_MODEL_SOURCES if (root / name).is_file()),
                 root / OPERATOR_MODEL_SOURCES[0])
    return parse_operator_schema(model.read_text())


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def sha(data):
    return hashlib.sha256(data).hexdigest()


def _launch_digests(rows):
    if not isinstance(rows, list) or not 1 <= len(rows) <= 128:
        raise ValueError("prebuilt launch component provenance")
    result = {}
    for row in rows:
        if (not isinstance(row, dict) or set(row) != {"file", "sha256"}
                or not isinstance(row["file"], str)
                or re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9.+_-]{0,127}", row["file"]) is None
                or not isinstance(row["sha256"], str)
                or re.fullmatch(r"[a-f0-9]{64}", row["sha256"]) is None
                or row["file"] in result):
            raise ValueError("prebuilt launch component provenance")
        result[row["file"]] = row["sha256"]
    return result


def _launch_notice_digests(notices):
    # The retained build combines exact installed texts under these public labels.
    parts = re.split(rb"\n=== notices/([A-Za-z0-9][A-Za-z0-9.+_-]{0,127}) ===\n", notices)
    if len(parts) < 3 or len(parts) % 2 != 1:
        raise ValueError("prebuilt launch component notices")
    rows = []
    for name, text in zip(parts[1::2], parts[2::2]):
        if not text.endswith(b"\n"):
            raise ValueError("prebuilt launch component notices")
        rows.append({"file": name.decode("ascii"), "sha256": sha(text[:-1])})
    return _launch_digests(rows)


def verify_launch_source(origin, binary, notices):
    """Validate the public, path-free projection of the retained build facts."""
    try:
        build = origin["build"]
        if (set(origin) != {"schema", "component", "artifact_sha256", "sources", "build"}
                or type(origin["schema"]) is not int or origin["schema"] != 1
                or origin["component"] != "installer_launch" or not binary.startswith(b"MZ")
                or origin["artifact_sha256"] != sha(binary)
                or set(origin["sources"]) != LAUNCH_SOURCES
                or any(not isinstance(value, str) or re.fullmatch(r"[a-f0-9]{64}", value) is None
                       for value in origin["sources"].values())
                or set(build) != {"target", "compiler", "compiler_version", "compiler_sha256",
                                  "flags", "invocation_sha256", "producer", "link_inputs",
                                  "notices", "notices_sha256"}
                or build["target"] != "x86_64-windows-gnu" or build["compiler"] != "zig c++"
                or not isinstance(build["compiler_version"], str)
                or re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:[-+][A-Za-z0-9._-]+)?",
                                build["compiler_version"]) is None
                or build["flags"] != list(LAUNCH_FLAGS)
                or set(build["producer"]) != {"source", "sha256"}
                or build["producer"]["source"] != LAUNCH_PRODUCER
                or any(not isinstance(value, str) or re.fullmatch(r"[a-f0-9]{64}", value) is None
                       for value in (build["compiler_sha256"], build["invocation_sha256"],
                                     build["producer"]["sha256"], build["notices_sha256"]))):
            raise ValueError("prebuilt launch component provenance")
        _launch_digests(build["link_inputs"])
        if (build["notices_sha256"] != sha(notices)
                or _launch_digests(build["notices"]) != _launch_notice_digests(notices)):
            raise ValueError("prebuilt launch component notices")
    except (KeyError, TypeError, AttributeError) as error:
        raise ValueError("prebuilt launch component provenance") from error


def public_launch_source(origin, binary, notices):
    """Project genuine build records; keep compiler/cache paths in the raw record."""
    try:
        build = origin["build"]
        if "arguments" not in build:
            verify_launch_source(origin, binary, notices)
            return origin
        args = build["arguments"]
        if (not isinstance(args, list) or len(args) != 13
                or args[:8] != ["c++", *LAUNCH_FLAGS[:7]]
                or args[9:12] != [*LAUNCH_FLAGS[7:], "-o"]
                or PurePosixPath(args[8]).name != "launch.cpp"
                or PurePosixPath(args[12]).name != "installer-launch.exe"
                or build["generated_service_define"] is not False
                or build["combined_notices"]["sha256"] != sha(notices)):
            raise ValueError("prebuilt launch component production recipe")
        public = {key: origin[key] for key in ("schema", "component", "artifact_sha256", "sources")}
        public["build"] = {key: build[key] for key in
                           ("target", "compiler", "compiler_version", "compiler_sha256")}
        public["build"].update({
            "flags": list(LAUNCH_FLAGS), "invocation_sha256": sha(canonical(args)),
            "producer": {key: build["producer"][key] for key in ("source", "sha256")},
            "link_inputs": [{"file": PurePosixPath(row["path"]).name, "sha256": row["sha256"]}
                            for row in build["link_inputs"]],
            "notices": [{"file": PurePosixPath(row["file"]).name, "sha256": row["sha256"]}
                        for row in build["notices"]],
            "notices_sha256": build["combined_notices"]["sha256"],
        })
        verify_launch_source(public, binary, notices)
        return public
    except (KeyError, TypeError, AttributeError) as error:
        raise ValueError("prebuilt launch component provenance") from error


def public_launch_kit(data):
    """Preserve kit bytes except its public launch record and that record's digest."""
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        names = archive.namelist()
        if not LAUNCH_MEMBERS.intersection(names):
            return data
        if not LAUNCH_MEMBERS <= set(names) or len(names) != len(set(names)):
            raise ValueError("prebuilt launch component roster")
        entries = archive.infolist()
        if (len(data) > 256 * 1024 * 1024 or len(entries) > 256
                or any(entry.file_size > 128 * 1024 * 1024 for entry in entries)
                or sum(entry.file_size for entry in entries) > 256 * 1024 * 1024):
            raise ValueError("prebuilt kit extent")
        origin_name = "runtime/installer-launch-source.json"
        if archive.getinfo(origin_name).file_size > 65536:
            raise ValueError("prebuilt launch component provenance extent")
        raw = archive.read(origin_name)
        recipe = json.loads(archive.read("recipe.json"))
        if recipe["files"].get(origin_name) != sha(raw):
            raise ValueError("prebuilt launch component provenance digest")
        origin = public_launch_source(json.loads(raw), archive.read("runtime/installer-launch.exe"),
                                      archive.read("licenses/installer-launch.txt"))
        if "arguments" not in json.loads(raw)["build"]:
            return data
        projected = canonical(origin)
        recipe["files"][origin_name] = sha(projected)
        output = io.BytesIO()
        with zipfile.ZipFile(output, "w") as target:
            for entry in archive.infolist():
                content = (projected if entry.filename == origin_name else canonical(recipe)
                           if entry.filename == "recipe.json" else archive.read(entry.filename))
                target.writestr(entry, content)
        return output.getvalue()


def path_ok(name):
    p = PurePosixPath(name)
    parts = p.parts
    if str(p) != name or p.is_absolute() or not parts or parts[0] != "usr" or any(
        x in ("", ".", "..", ".git", "target", "build", "__pycache__") for x in parts
    ):
        raise ValueError("package destination outside controlled usr tree")
    if not (name.startswith("usr/bin/") or name.startswith("usr/lib/linux-vst-bridge/")
            or name.startswith("usr/share/doc/linux-vst-bridge-beta/")
            or name in (SYSTEM_DESKTOP, ADOPTION_MANIFEST)):
        raise ValueError("package destination not admitted")
    if any(x in name.lower() for x in ("credential", "activation", "private-key", "installer.exe")):
        raise ValueError("sensitive destination name")


def validate_file_role(name, kind, mode, component, *, generated=False):
    """One destination/role law for builder inputs and retained release rosters."""
    path_ok(name)
    if (not isinstance(kind, str) or not isinstance(mode, str)
            or not isinstance(component, str)
            or re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9.+_-]{0,80}", component) is None
            or mode != ("0555" if kind in EXECUTABLE_KINDS else "0444")):
        raise ValueError("file mode, role or component")
    if name in (ADOPTION_MANIFEST, SYSTEM_DESKTOP):
        expected = "adoption_manifest" if name == ADOPTION_MANIFEST else "system_desktop"
        if not generated or kind != expected or component != PACKAGE:
            raise ValueError("generated package authority differs")
    elif kind in ("adoption_manifest", "system_desktop"):
        raise ValueError("generated package authority destination differs")
    elif name in REQUIRED or name in SOURCE_REQUIRED:
        if {**REQUIRED, **SOURCE_REQUIRED}[name] != kind:
            raise ValueError("required package role")
    else:
        prefix = OPTIONAL_PREFIXES.get(kind)
        if prefix is None or not name.startswith(prefix):
            raise ValueError("file role and destination disagree")
    if kind == "preparation_kit" and name != KIT_DESTINATION:
        raise ValueError("preparation kit destination")
    if (kind != "license" and PurePosixPath(name).suffix in SOURCE_SUFFIXES
            and name not in SOURCE_SUPERVISOR_PATHS):
        raise ValueError("proprietary source in binary package")


def validate_release_roster(manifest):
    """Refuse a signable manifest that the PKG0 builder could never produce."""
    rows = manifest.get("files")
    if not isinstance(rows, list) or not 1 <= len(rows) <= 100002:
        raise ValueError("release file count")
    seen = {}
    for row in rows:
        if (not isinstance(row, dict)
                or set(row) != {"destination", "sha256", "size", "mode", "kind", "component"}
                or not isinstance(row["destination"], str)
                or not isinstance(row["sha256"], str)
                or re.fullmatch(r"[0-9a-f]{64}", row["sha256"]) is None
                or type(row["size"]) is not int or not 0 <= row["size"] <= 2_000_000_000):
            raise ValueError("release file row")
        name = row["destination"]
        if name in seen:
            raise ValueError("duplicate release roster")
        validate_file_role(name, row["kind"], row["mode"], row["component"], generated=True)
        seen[name] = row
    required = SOURCE_REQUIRED if source_supervisor(seen) else REQUIRED
    if not (set(required) | {ADOPTION_MANIFEST, SYSTEM_DESKTOP}) <= set(seen):
        raise ValueError("required package roster absent")
    for kind in ("proxy", "fixture", "profile", "license"):
        if not any(row["kind"] == kind for row in rows):
            raise ValueError("required package component absent")
    desktop = seen[SYSTEM_DESKTOP]
    if desktop["sha256"] != sha(SYSTEM_DESKTOP_BYTES) or desktop["size"] != len(SYSTEM_DESKTOP_BYTES):
        raise ValueError("system desktop identity differs")
    return seen


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
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        if archive.getinfo("recipe.json").file_size > 65536:
            raise ValueError("preparation kit recipe extent")
        if json.loads(archive.read("recipe.json")).get("schema") in (3,4):
            return verify_prebuilt_kit(data, source_head, host_sha256, source_sha256)
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
                or recipe["sdk"] != KIT_SDK
                or recipe["sdk_runtime"] != KIT_SDK_RUNTIME):
            raise ValueError("preparation kit recipe")
        required = {"libap2_backend.a", "runtime/host.exe",
                    "runtime/host-source-manifest.json", "tools/mf3/native_builder.py",
                    "tools/ap8_descriptor.py", "CMakeLists.txt",
                    "native-vst3-proxy/CMakeLists.txt",
                    "native-vst3-proxy/source/processor.cpp",
                    "native-vst3-proxy/source/factory.cpp",
                    "native-vst3-proxy/source/processor.h",
                    "native-vst3-proxy/include/ap2_backend.h",
                    "vst-state/stream.h"}
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


def verify_prebuilt_kit(data, source_head, host_sha256, source_sha256):
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        entries=archive.infolist();names=[e.filename for e in entries]
        fixed={"recipe.json","libap2_backend.a","tools/mf3/native_builder.py",
            "tools/ap8_descriptor.py","runtime/host.exe","runtime/host-source-manifest.json",
            "prebuilt/index.json","licenses/vst3sdk.txt","licenses/base.txt",
            "licenses/pluginterfaces.txt","licenses/public.sdk.txt"}
        if not fixed <= set(names) or len(names)!=len(set(names)) or len(names)>256:
            raise ValueError("prebuilt kit roster")
        recipe=json.loads(archive.read("recipe.json"))
        if recipe.get("schema") == 4: fixed.add("prebuilt/engine.so")
        helpers={"runtime/lvb-direct-wait.dll","runtime/x86_64-windows/lvb-direct-wait.dll",
                 "runtime/x86_64-unix/lvb-direct-wait.so","runtime/direct-audio-helper.json"}
        present=helpers & set(names)
        if present:
            if recipe.get("schema") != 4 or present != helpers:
                raise ValueError("prebuilt direct helper roster")
            fixed.update(helpers)
        launch=LAUNCH_MEMBERS
        launch_present=launch & set(names)
        if launch_present:
            if recipe.get("schema")!=4 or launch_present!=launch:
                raise ValueError("prebuilt launch component roster")
            fixed.update(launch)
        total=0
        for entry in entries:
            name=entry.filename
            if (entry.is_dir() or stat.S_IFMT(entry.external_attr>>16) not in (0,stat.S_IFREG)
                or entry.file_size>128*1024*1024
                or name not in fixed and re.fullmatch(r"prebuilt/[0-9A-F]{32}-[0-9a-f]{64}\.(so|h)",name) is None):
                raise ValueError("prebuilt kit entry")
            total+=entry.file_size
        if total>256*1024*1024:raise ValueError("prebuilt kit extent")
        recipe=json.loads(archive.read("recipe.json"))
        if (set(recipe)!={"schema","source_commit","sdk","sdk_runtime","files"}
            or recipe["schema"] not in (3,4) or recipe["source_commit"]!=source_head
            or recipe["sdk"]!=KIT_SDK or recipe["sdk_runtime"]!=KIT_SDK_RUNTIME
            or set(recipe["files"])!=set(names)-{"recipe.json"}):
            raise ValueError("prebuilt kit recipe")
        for name,expected in recipe["files"].items():
            content=archive.read(name)
            if not re.fullmatch(r"[0-9a-f]{64}",expected) or sha(content)!=expected or any(x in content for x in SECRET_MARKERS):
                raise ValueError("prebuilt kit file digest")
        if recipe["files"]["runtime/host.exe"]!=host_sha256 or recipe["files"]["runtime/host-source-manifest.json"]!=source_sha256:
            raise ValueError("prebuilt kit host pair")
        if present:
            helper=json.loads(archive.read("runtime/direct-audio-helper.json"))
            expected={name.removeprefix("runtime/"):recipe["files"][name]
                      for name in helpers if name != "runtime/direct-audio-helper.json"}
            shim=archive.read("runtime/lvb-direct-wait.dll")
            helper_keys={"schema","abi","host_sha256","runner_wine_revision","files"}
            common=helper.get("schema")==2
            if common:helper_keys.add("runner_build_sha256")
            if (set(helper)!=helper_keys
                or type(helper["schema"]) is not int or helper["schema"] not in (1,2)
                or type(helper["abi"]) is not int or helper["abi"]!=1
                or helper["host_sha256"]!=host_sha256
                or (not common and helper["runner_wine_revision"]!="46b29104e3741fe23bf5e2547196a253aab88c89")
                or (common and (not isinstance(helper["runner_wine_revision"],str)
                    or re.fullmatch("[0-9a-f]{40}",helper["runner_wine_revision"]) is None
                    or not isinstance(helper["runner_build_sha256"],str)
                    or re.fullmatch("[0-9a-f]{64}",helper["runner_build_sha256"]) is None))
                or helper["files"]!=expected
                or not shim.startswith(b"MZ") or b"Wine builtin DLL" not in shim[:128]
                or shim!=archive.read("runtime/x86_64-windows/lvb-direct-wait.dll")
                or not archive.read("runtime/x86_64-unix/lvb-direct-wait.so").startswith(b"\x7fELF")):
                raise ValueError("prebuilt direct helper binding")
        if launch_present:
            if archive.getinfo("runtime/installer-launch-source.json").file_size > 65536:
                raise ValueError("prebuilt launch component provenance extent")
            binary=archive.read("runtime/installer-launch.exe")
            origin=json.loads(archive.read("runtime/installer-launch-source.json"))
            verify_launch_source(origin, binary, archive.read("licenses/installer-launch.txt"))
        index=json.loads(archive.read("prebuilt/index.json"))
        if recipe["schema"] == 4:
            if (set(index) !=
                    {"schema","engine","engine_sha256","descriptor_schema","maximum_bridge_frames","native_sources","audio_completion_contract","loaded_engine_admission_contract"}
                or ("audio_completion_contract" in index and (type(index["audio_completion_contract"]) is not int or index["audio_completion_contract"] != 1))
                or type(index["loaded_engine_admission_contract"]) is not int
                or index["loaded_engine_admission_contract"] != 1
                or index["schema"]!=3 or index["descriptor_schema"]!=1
                or index["maximum_bridge_frames"]!=1024 or index["engine"]!="prebuilt/engine.so"
                or recipe["files"][index["engine"]]!=index["engine_sha256"]
                or not archive.read(index["engine"]).startswith(b"\x7fELF") or set(names)!=fixed):
                raise ValueError("reusable engine index")
            return
        if set(index)!={"schema","proxies","native_sources"} or index["schema"] not in (1,2) or not 1<=len(index["proxies"])<=64:
            raise ValueError("prebuilt proxy index")
        selected=set();expected_names=set(fixed)
        for proxy in index["proxies"]:
            fields={"class_id","module_sha256","file","descriptor","descriptor_sha256","native_sha256"}
            if index["schema"]==2:fields.add("maximum_bridge_frames")
            if set(proxy)!=fields or index["schema"]==2 and proxy["maximum_bridge_frames"]!=1024:
                raise ValueError("prebuilt proxy row")
            key=(proxy["class_id"],proxy["module_sha256"])
            if key in selected or re.fullmatch(r"[0-9A-F]{32}",key[0]) is None or re.fullmatch(r"[0-9a-f]{64}",key[1]) is None:
                raise ValueError("prebuilt proxy identity")
            selected.add(key);stem="prebuilt/"+key[0]+"-"+key[1]
            if (proxy["file"]!=stem+".so" or proxy["descriptor"]!=stem+".h"
                or recipe["files"].get(proxy["file"])!=proxy["native_sha256"]
                or recipe["files"].get(proxy["descriptor"])!=proxy["descriptor_sha256"]
                or not archive.read(proxy["file"]).startswith(b"\x7fELF")):
                raise ValueError("prebuilt proxy bytes")
            expected_names.update((proxy["file"],proxy["descriptor"]))
        if set(names)!=expected_names:raise ValueError("prebuilt kit extra artifact")

def verify_kit_source(data, source_root, source_head, source_tree):
    """Bind a releasable kit's complete source roster to one clean Git tree."""
    source_root = Path(source_root).resolve(strict=True)

    def git(*args):
        return subprocess.check_output(["git", "-C", str(source_root), *args])

    if (Path(git("rev-parse", "--show-toplevel").decode().strip()) != source_root
            or git("rev-parse", "HEAD").decode().strip() != source_head
            or git("rev-parse", "HEAD^{tree}").decode().strip() != source_tree
            or git("status", "--porcelain")):
        raise ValueError("preparation kit source head/tree differs")
    names = {name for name in git("ls-files", "-z", "--", *KIT_SOURCE_ARGS)
             .decode().split("\0") if name}
    if len(names) > 512:
        raise ValueError("preparation kit source roster bound")
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        recipe = json.loads(archive.read("recipe.json"))
        if recipe["schema"] in (3,4):
            native_names={name for name in git("ls-files","-z","--",*KIT_SOURCE_ARGS[:5],"native-audio-client").decode().split("\0") if name}
            if recipe["schema"] == 4:
                native_names.update(name for name in git("ls-files","-z","--","plugin-descriptor").decode().split("\0") if name)
            index=json.loads(archive.read("prebuilt/index.json"))
            if index["native_sources"]!={name:sha((source_root/name).read_bytes()) for name in native_names}:
                raise ValueError("prebuilt native source differs")
            for name in ("tools/mf3/native_builder.py","tools/ap8_descriptor.py"):
                if archive.read(name)!=(source_root/name).read_bytes():
                    raise ValueError("prebuilt preparation source differs")
            if "runtime/installer-launch-source.json" in archive.namelist():
                origin=json.loads(archive.read("runtime/installer-launch-source.json"))
                verify_launch_source(origin, archive.read("runtime/installer-launch.exe"),
                                     archive.read("licenses/installer-launch.txt"))
                inputs = {**origin["sources"], LAUNCH_PRODUCER: origin["build"]["producer"]["sha256"]}
                for name,expected in inputs.items():
                    relative=PurePosixPath(name)
                    path=source_root/name
                    if (relative.is_absolute() or str(relative)!=name or ".." in relative.parts
                        or not re.fullmatch("[a-f0-9]{64}",expected)
                        or name not in git("ls-files","-z","--",name).decode().split("\0")
                        or path.is_symlink() or not path.is_file()
                        or sha(path.read_bytes())!=expected
                        or sha(git("show",source_head+":"+name))!=expected):
                        raise ValueError("prebuilt launch component source differs")
            return
        if set(recipe["files"]) != names | KIT_GENERATED:
            raise ValueError("preparation kit source roster differs")
        for name in names:
            path = source_root / name
            md = path.lstat()
            if not stat.S_ISREG(md.st_mode) or path.is_symlink() or archive.read(name) != path.read_bytes():
                raise ValueError("preparation kit source bytes differ")
    if (git("rev-parse", "HEAD").decode().strip() != source_head
            or git("rev-parse", "HEAD^{tree}").decode().strip() != source_tree
            or git("status", "--porcelain")):
        raise ValueError("preparation kit source changed during verification")


def verify_kit_backend(data, source_root):
    """Rebuild the registered backend before a release key signs its archive."""
    source_root = Path(source_root).resolve(strict=True)
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        expected = sha(archive.read("libap2_backend.a"))
        recipe = json.loads(archive.read("recipe.json"))
        prebuilt_index = json.loads(archive.read("prebuilt/index.json")) if recipe["schema"] in (3,4) else None
    compiler = subprocess.check_output(
        ["rustup", "which", "--toolchain", KIT_RUST_TOOLCHAIN, "rustc"], text=True).strip()
    with tempfile.TemporaryDirectory(prefix="lvb-pkg1-backend-") as target:
        env = {key: os.environ[key] for key in ("PATH", "HOME", "CARGO_HOME", "RUSTUP_HOME")
               if key in os.environ}
        env.update({"RUSTC": compiler, "RUSTFLAGS": "-C relocation-model=pic",
                    "CARGO_TARGET_DIR": target, "CARGO_INCREMENTAL": "0"})
        process = subprocess.Popen(
            ["rustup", "run", KIT_RUST_TOOLCHAIN, "cargo", "build", "--manifest-path",
             "native-vst3-proxy/backend/Cargo.toml", "--release", "--locked", "--offline",
             "--target", "x86_64-unknown-linux-gnu", "--features", "registered"],
            cwd=source_root, env=env, stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL, start_new_session=True)
        try:
            code = process.wait(timeout=600)
        except subprocess.TimeoutExpired as error:
            try:
                os.killpg(process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                process.wait()
            raise ValueError("preparation kit backend rebuild deadline") from error
        if code:
            raise ValueError("preparation kit backend rebuild failed")
        built = Path(target) / "x86_64-unknown-linux-gnu/release/libap2_backend.a"
        if not built.is_file() or built.is_symlink() or sha(built.read_bytes()) != expected:
            raise ValueError("preparation kit backend source differs")
        if prebuilt_index is not None:
            sdk = Path.home()/".cache/linux-vst-bridge/dependencies/vst3sdk"/KIT_SDK
            if not sdk.is_dir():
                raise ValueError("release build machine requires the pinned SDK")
            with zipfile.ZipFile(io.BytesIO(data)) as archive:
                for component in ('', 'base', 'pluginterfaces', 'public.sdk'):
                    if archive.read('licenses/'+(component or 'vst3sdk')+'.txt') != (sdk/component/'LICENSE.txt').read_bytes():
                        raise ValueError('prebuilt SDK license differs')
                if prebuilt_index["schema"] == 3:
                    build=Path(target)/"reusable-engine"
                    subprocess.run(["cmake","-S",str(source_root),"-B",str(build),"-G","Ninja",
                        "-DCMAKE_BUILD_TYPE=Release","-DAP2_BUILD_ONLY=ON","-DLVB_REUSABLE_ENGINE=ON",
                        "-DVST3_SDK_ROOT="+str(sdk),"-DAP2_RUST_LIBRARY="+str(built)],check=True,timeout=120)
                    subprocess.run(["cmake","--build",str(build),"--target","CommercialInstrumentBridge","-j","2"],check=True,timeout=600)
                    native=build/"VST3/Release/CommercialInstrumentBridge.vst3/Contents/x86_64-linux/CommercialInstrumentBridge.so"
                    if sha(native.read_bytes())!=prebuilt_index["engine_sha256"]:
                        raise ValueError("reusable engine rebuild differs")
                    return
                for number, proxy in enumerate(prebuilt_index["proxies"]):
                    metadata=Path(target)/("descriptor-"+str(number));metadata.mkdir()
                    header=metadata/"ap8_descriptor.h"
                    header.write_bytes(archive.read(proxy["descriptor"]))
                    build=Path(target)/("proxy-"+str(number))
                    subprocess.run(["cmake","-S",str(source_root),"-B",str(build),"-G","Ninja",
                        "-DCMAKE_BUILD_TYPE=Release","-DAP2_BUILD_ONLY=ON",
                        "-DVST3_SDK_ROOT="+str(sdk),"-DAP2_RUST_LIBRARY="+str(built),
                        "-DAP8_DESCRIPTOR="+str(header)],check=True,timeout=120)
                    subprocess.run(["cmake","--build",str(build),"--target","CommercialInstrumentBridge","-j","2"],check=True,timeout=600)
                    native=build/"VST3/Release/CommercialInstrumentBridge.vst3/Contents/x86_64-linux/CommercialInstrumentBridge.so"
                    if sha(native.read_bytes())!=proxy["native_sha256"]:
                        raise ValueError("prebuilt native rebuild differs")


def validate_frontend_build(record, source_head, source_tree, artifact_sha256):
    """The package's Bridge source and the additional UI source stay distinct.

    This is build provenance, not a second runtime selection mechanism. The
    signed adoption record continues to bind the actual manager/frontend pair.
    """
    expected = {"schema", "bridge_source", "frontend_source", "target", "binary", "features",
                "portable_public_key", "portable_key_class", "artifact_sha256"}
    if not isinstance(record, dict) or set(record) != expected or record["schema"] != 1:
        raise ValueError("composite frontend build schema")
    if record["bridge_source"] != {"head": source_head, "tree": source_tree}:
        raise ValueError("paired composite frontend Bridge generation differs")
    private = record["frontend_source"]
    if (not isinstance(private, dict)
            or set(private) != {"head", "tree", "manifest", "manifest_sha256", "cargo_lock_sha256"}
            or any(not isinstance(private[k], str) or not re.fullmatch(r"[0-9a-f]{40}", private[k])
                   for k in ("head", "tree"))
            or any(not isinstance(private[k], str) or not re.fullmatch(r"[0-9a-f]{64}", private[k])
                   for k in ("manifest_sha256", "cargo_lock_sha256"))):
        raise ValueError("composite frontend source identity")
    name = private["manifest"]
    if (not isinstance(name, str) or len(name) > 256 or "\\" in name
            or PurePosixPath(name).is_absolute() or ".." in PurePosixPath(name).parts
            or PurePosixPath(name).as_posix() != name or PurePosixPath(name).name != "Cargo.toml"):
        raise ValueError("composite frontend manifest name")
    if (record["target"] != "x86_64-unknown-linux-gnu"
            or record["binary"] != "linux-audio-compatibility-manager"
            or record["features"] != ["live"] or record["portable_key_class"] != "internal_test"
            or not isinstance(record["portable_public_key"], str)
            or not re.fullmatch(r"[0-9a-f]{64}", record["portable_public_key"])
            or record["artifact_sha256"] != artifact_sha256):
        raise ValueError("composite frontend artifact/build identity")


def validate(spec, source_root=None):
    base = {"schema", "version", "source_head", "source_tree", "operator_schema",
            "external_runtime", "files"}
    if set(spec) != base or spec["schema"] not in (1, 2, 3):
        raise ValueError("PKG0 input schema")
    if (type(spec["operator_schema"]) is not int
            or spec["operator_schema"] != declared_operator_schema(source_root)):
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
        if item.get("kind") == "frontend" and "frontend_build" in item:
            expected.add("frontend_build")
            validate_frontend_build(item["frontend_build"], spec["source_head"],
                                    spec["source_tree"], item["sha256"])
        elif item.get("kind") in ("manager", "frontend"):
            expected |= {"build_head", "build_tree"}
            if ((item.get("build_head"), item.get("build_tree"))
                    != (spec["source_head"], spec["source_tree"])):
                raise ValueError("paired manager/frontend build generation differs")
        if set(item) != expected:
            raise ValueError("file input schema")
        name = item["destination"]
        if name in (ADOPTION_MANIFEST, SYSTEM_DESKTOP) or name in seen:
            raise ValueError("duplicate or reserved package destination")
        if not re.fullmatch(r"[0-9a-f]{64}", item["sha256"]):
            raise ValueError("file digest syntax")
        validate_file_role(name, item["kind"], item["mode"], item["component"])
        seen[name] = item
    source = source_supervisor(seen)
    if source != (spec["schema"] == 3):
        raise ValueError("supervisor format differs from package schema")
    if any(name not in seen for name in (SOURCE_REQUIRED if source else REQUIRED)):
        raise ValueError("required binary/document missing")
    if (KIT_DESTINATION in seen) != (spec["schema"] >= 2):
        raise ValueError("preparation kit required by package schema")
    if not any(x["kind"] == "proxy" for x in spec["files"]):
        raise ValueError("native proxy absent")
    if not any(x["kind"] == "fixture" for x in spec["files"]):
        raise ValueError("self-test fixture absent")
    if not any(x["kind"] == "profile" for x in spec["files"]):
        raise ValueError("supported profile absent")
    if not any(x["kind"] == "license" for x in spec["files"]):
        raise ValueError("third-party license text absent")
    composite = seen["usr/bin/linux-audio-compatibility-manager"].get("frontend_build")
    if composite is not None:
        compliance = json.loads(file_bytes(seen[
            "usr/share/doc/linux-vst-bridge-beta/COMPLIANCE_MANIFEST.json"]))
        if (compliance.get("frontend_build") != composite
                or compliance.get("classification") != "internal_test_only"
                or compliance.get("release_authorized") is not False):
            raise ValueError("composite frontend provenance not bound by signed compliance file")
    return seen


def _build(spec, output, epoch, source_root=None):
    files = validate(spec, source_root)
    if output.exists():
        raise ValueError("output already exists")
    output.mkdir(parents=True, mode=0o700)
    payload = output / "payload.tar"
    roster = []
    supervisor_headers = {}
    with tarfile.open(payload, "w", format=tarfile.PAX_FORMAT) as archive:
        host_sha256 = files["usr/lib/linux-vst-bridge/host/bridge-host.exe"]["sha256"]
        source_sha256 = files["usr/lib/linux-vst-bridge/host/source-manifest.json"]["sha256"]
        for name, item in sorted(files.items()):
            data = file_bytes(item)
            if item["kind"] == "preparation_kit":
                verify_kit(data, spec["source_head"], host_sha256, source_sha256)
                if source_root is not None:
                    verify_kit_source(data, source_root, spec["source_head"], spec["source_tree"])
            if item["kind"] in ("manager", "frontend", "proxy") and not data.startswith(b"\x7fELF"):
                raise ValueError("Linux executable format")
            if item["kind"] in ("windows_host", "fixture") and not data.startswith(b"MZ"):
                raise ValueError("Windows executable format")
            if name in SOURCE_SUPERVISOR_PATHS:
                compile(data, name, "exec")
                if source_root is not None:
                    exact = Path(source_root) / "bridge-manager/runtime" / PurePosixPath(name).name
                    if data != exact.read_bytes():
                        raise ValueError("supervisor differs from exact product source")
            if name in SUPERVISOR_PATHS:
                supervisor_headers[name] = data[:16]
            add_bytes(archive, name, data, int(item["mode"], 8), epoch)
            roster.append({"destination": name, "sha256": sha(data), "size": len(data),
                           "mode": item["mode"], "kind": item["kind"], "component": item["component"]})
        adopted_names = SOURCE_ADOPTED if spec["schema"] == 3 else (ADOPTED_WITH_KIT if spec["schema"] == 2 else ADOPTED)
        adopted = [{"name": adopted_names[name], "sha256": files[name]["sha256"],
                    "size": len(file_bytes(files[name]))} for name in adopted_names]
        adoption_record = {"schema": spec["schema"], "package": PACKAGE, "version": spec["version"],
                              "pkgrel": PKGREL,
                              "source_head": spec["source_head"],
                              "source_tree": spec["source_tree"],
                              "operator_schema": spec["operator_schema"],
                              "files": adopted, "external_runtime": spec["external_runtime"]}
        if spec["schema"] == 3:
            helper_name = "usr/lib/linux-vst-bridge/supervisor/session.py"
            capability = library_locations_capability(file_bytes(files[helper_name]))
            if capability is not None:
                adoption_record["library_locations"] = capability
        adoption = canonical(adoption_record)
        add_bytes(archive, ADOPTION_MANIFEST, adoption, 0o444, epoch)
        roster.append({"destination": ADOPTION_MANIFEST, "sha256": sha(adoption),
                       "size": len(adoption), "mode": "0444",
                       "kind": "adoption_manifest", "component": PACKAGE})
        # Package installation exposes an explicit first-run UI. Selection of
        # user-owned software remains a separate, guarded package-adopt action.
        add_bytes(archive, SYSTEM_DESKTOP, SYSTEM_DESKTOP_BYTES, 0o444, epoch)
        roster.append({"destination": SYSTEM_DESKTOP, "sha256": sha(SYSTEM_DESKTOP_BYTES),
                       "size": len(SYSTEM_DESKTOP_BYTES), "mode": "0444",
                       "kind": "system_desktop", "component": PACKAGE})
    manifest = {"schema": 1, "package": PACKAGE, "version": spec["version"],
                "pkgrel": PKGREL,
                "source_head": spec["source_head"], "source_tree": spec["source_tree"],
                "payload_sha256": sha(payload.read_bytes()), "files": roster}
    validate_release_roster(manifest)
    python_minor = SOURCE_PYTHON_RANGE if spec["schema"] == 3 else supervisor_python_minor(supervisor_headers)
    (output / "RELEASE_MANIFEST.json").write_bytes(canonical(manifest))
    pkgbuild = f'''pkgname={PACKAGE}
pkgver={spec["version"]}
pkgrel={PKGREL}
pkgdesc="Private Linux VST Bridge beta for native Linux DAWs"
arch=('x86_64')
url='https://github.com/kasselvania/Linux-VST-bridge'
license=('LicenseRef-Proprietary')
depends=({' '.join(repr(dependency) for dependency in package_dependencies(python_minor))})
options=('!strip' '!debug' '!lto')
source=('payload.tar')
sha256sums=('{manifest["payload_sha256"]}')
package() {{
  bsdtar -xf "$srcdir/payload.tar" -C "$pkgdir"
}}
'''
    (output / "PKGBUILD").write_text(pkgbuild)
    return manifest


def build(spec, output, epoch, source_root=None):
    output = Path(output)
    if output.exists():
        raise ValueError("output already exists")
    temporary = output.with_name(output.name + ".partial-" + secrets.token_hex(8))
    try:
        manifest = _build(spec, temporary, epoch, source_root)
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
    head = subprocess.check_output(["git", "-C", str(source), "rev-parse", "HEAD"], text=True).strip()
    tree = subprocess.check_output(["git", "-C", str(source), "rev-parse", "HEAD^{tree}"], text=True).strip()
    if subprocess.check_output(["git", "-C", str(source), "status", "--porcelain"]):
        raise ValueError("commit package source before assembly")
    spec = json.loads(a.spec.read_bytes())
    if (spec.get("source_head"), spec.get("source_tree")) != (head, tree):
        raise ValueError("package source head/tree differs")
    epoch = int(subprocess.check_output(["git", "-C", str(source), "show", "-s", "--format=%ct", "HEAD"]).strip())
    print(json.dumps(build(spec, a.output, epoch, source), sort_keys=True))


if __name__ == "__main__":
    main()

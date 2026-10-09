#!/usr/bin/env python3
"""Seal a corrected x64 Wine component with its complete corresponding source.

No runner or prefix is adopted. GE and SLR remain separately acquired upstream.
"""
import argparse
import gzip
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import shutil
import struct
import tarfile

import build

GE_SHA = "c5448b76a230384e2d7bc6beb5ccb97bafb7e2c3b6c527cb03a1a546bbcb00a0"
GE_ROOT = "GE-Proton11-7-x86_64"
DLL_PATH = "files/lib/wine/x86_64-windows/uiautomationcore.dll"
ORIGINAL_DLL_SHA = "d4362453451e01c937fba7272e1644164ef2498a861216b00696161fe43dbdcb"


def identity(path):
    return dict(path=path.name, sha256=build.sha(path), size=path.stat().st_size)


def pe_imports(path):
    """Read only PE architecture/import names; reject malformed/out-of-bounds data."""
    data = path.read_bytes()
    if len(data) < 64 or data[:2] != b"MZ":
        raise ValueError("PE header missing")
    off = struct.unpack_from("<I", data, 0x3c)[0]
    if off + 24 > len(data) or data[off:off+4] != b"PE\0\0":
        raise ValueError("PE signature missing")
    machine, count = struct.unpack_from("<HH", data, off + 4)
    opt_size = struct.unpack_from("<H", data, off + 20)[0]
    opt = off + 24
    if machine != 0x8664 or opt_size < 128 or opt + opt_size + count * 40 > len(data):
        raise ValueError("component must be an intact x64 PE image")
    if struct.unpack_from("<H", data, opt)[0] != 0x20b:
        raise ValueError("component must be PE32+")
    sections = []
    for index in range(count):
        section = opt + opt_size + index * 40
        virtual_size, rva, raw_size, raw = struct.unpack_from("<IIII", data, section + 8)
        if raw + raw_size > len(data):
            raise ValueError("PE section extent")
        sections.append((rva, max(virtual_size, raw_size), raw, raw_size))

    def address(rva, extent):
        for start, length, raw, size in sections:
            if start <= rva < start + length and rva - start + extent <= size:
                return raw + rva - start
        raise ValueError("PE import address outside file")

    imports, extent = struct.unpack_from("<II", data, opt + 120)
    if not imports or not 20 <= extent <= len(data):
        raise ValueError("PE imports missing or excessive")
    names = []
    for index in range(min(128, extent // 20)):
        row = struct.unpack_from("<IIIII", data, address(imports + index * 20, 20))
        if not any(row):
            return sorted(set(names))
        start = address(row[3], 1)
        end = data.find(b"\0", start, min(len(data), start + 256))
        if end < 0:
            raise ValueError("PE import name extent")
        name = data[start:end].decode("ascii").lower()
        if not name.endswith(".dll") or "/" in name or "\\" in name:
            raise ValueError("PE import name invalid")
        names.append(name)
    raise ValueError("PE imports unterminated")


def add_bytes(tar, name, data, mode=0o644):
    info = tarfile.TarInfo(name)
    info.size, info.mode = len(data), mode
    tar.addfile(info, io.BytesIO(data))


def source_archive(output, built, receipt):
    # Use the pinned pristine source, replacing precisely the three changed
    # files. Generator caches, object files and diagnostics are never sources.
    prefix = "wine-" + build.WINE + "/"
    changed = {name: (built / "wine" / name).read_bytes() for name in [
        "dlls/uiautomationcore/uia_client.c", "dlls/uiautomationcore/tests/Makefile.in"]}
    guard = (built / "wine/dlls/uiautomationcore/tests/guard_provider.c").read_bytes()
    for name, digest in receipt["source_inputs"].items():
        if build.sha(built / "wine" / name) != digest:
            raise ValueError("consumed source changed")
    recipe = built / "build.sh"
    if build.sha(recipe) != receipt["build_script_sha256"]:
        raise ValueError("consumed build recipe changed")
    if build.sha(built / "wine.tar.gz") != build.ARCHIVE_SHA:
        raise ValueError("source archive changed")
    with output.open("xb") as raw, gzip.GzipFile(fileobj=raw, mode="wb", mtime=0) as gz:
        with tarfile.open(fileobj=gz, mode="w|") as out, tarfile.open(built / "wine.tar.gz") as original:
            seen = set()
            for member in original:
                if member.name == prefix[:-1]:
                    continue
                if not member.name.startswith(prefix):
                    raise ValueError("source archive root")
                relative = member.name[len(prefix):]
                if PurePosixPath(relative).is_absolute() or ".." in PurePosixPath(relative).parts:
                    raise ValueError("source archive path")
                seen.add(relative)
                if relative in changed:
                    add_bytes(out, "wine/" + relative, changed[relative])
                else:
                    member.name = "wine/" + relative
                    member.uid = member.gid = member.mtime = 0
                    member.uname = member.gname = ""
                    member.pax_headers = {}
                    out.addfile(member, original.extractfile(member) if member.isfile() else None)
            if not set(changed).issubset(seen):
                raise ValueError("modified source missing")
            add_bytes(out, "wine/dlls/uiautomationcore/tests/guard_provider.c", guard)
            add_bytes(out, "build.sh", recipe.read_bytes())
            add_bytes(out, "SDK.txt", (build.SDK + "\n").encode())


def package(built, ge_archive, output):
    if build.sha(ge_archive) != GE_SHA:
        raise ValueError("GE base archive changed")
    receipt = json.loads((built / "build-receipt.json").read_text())
    if (receipt["wine_commit"] != build.WINE or receipt["patch_sha256"] != build.PATCH_SHA
        or receipt["guard_source_sha256"] != build.GUARD_SHA or receipt["sdk_image"] != build.SDK):
        raise ValueError("build inputs changed")
    paths = {
        "uiautomationcore.dll": built / "obj/dlls/uiautomationcore/x86_64-windows/uiautomationcore.dll",
        "uiautomationcore_test.exe": built / "obj/dlls/uiautomationcore/tests/x86_64-windows/uiautomationcore_test.exe",
    }
    for name, path in paths.items():
        if build.sha(path) != receipt["artifacts"][name]:
            raise ValueError("built component changed")
    imports = {name: pe_imports(path) for name, path in paths.items()}
    with tarfile.open(ge_archive) as archive:
        preimage = archive.extractfile(GE_ROOT + "/" + DLL_PATH).read()
        if hashlib.sha256(preimage).hexdigest() != ORIGINAL_DLL_SHA:
            raise ValueError("GE UIA preimage changed")
        dlls = {PurePosixPath(m.name).name.lower() for m in archive
                if m.name.startswith(GE_ROOT + "/files/lib/wine/x86_64-windows/")}
    if any(not set(names).issubset(dlls) for names in imports.values()):
        raise ValueError("component imports unavailable in the exact GE runtime")
    output.mkdir(mode=0o700)
    component = output / "uia-guard"
    component.mkdir(mode=0o700)
    for name, path in paths.items():
        shutil.copyfile(path, component / name)
    for name, digest in {"build.py": receipt["builder_sha256"],
                         "guard_provider.c": receipt["guard_source_sha256"],
                         "uia-null-provider.patch": receipt["patch_sha256"]}.items():
        path = built / name
        if build.sha(path) != digest:
            raise ValueError("consumed construction input changed")
        shutil.copyfile(path, component / name)
    shutil.copyfile(Path(__file__).with_name("COPYING.LIB"), component / "COPYING.LIB")
    source_archive(component / "wine-source.tar.gz", built, receipt)
    notices = (
        "Wine x64 UI Automation null-provider correction, 2026-10-09\n"
        "uiautomationcore.dll, the matched Wine test and Wine source: LGPL-2.1-or-later.\n"
        "The complete patched Wine source is wine-source.tar.gz; original copyright and\n"
        "license notices are retained. Modified uia_client.c, tests/Makefile.in and the new\n"
        "guard_provider.c identify the change and date. COPYING.LIB carries the license.\n"
        "The archive includes build.sh and SDK.txt to compile from that source directly;\n"
        "extract it into a fresh /work volume and run build.sh in the pinned SDK container.\n"
        "Use two CPUs (cpuset 2,3), 4 GiB memory/no extra swap and 256 PIDs; make uses -j2.\n"
        "build.py, the exact patch and test source also reproduce from the pinned Wine input.\n"
        "Wine commit: " + build.WINE + "\nSDK: " + build.SDK + "\n"
        "GE-Proton11-7 source commit: c191f35dcebbeccfacd3b4c6f6eea026e588c1c2\n"
        "Wine-staging revision: 6cc805ea57132eeaf44764e9213823c9b8d0d300\n"
        "The exact GE preparation series and pinned Wine-staging patches were checked\n"
        "against the generated dependency closure for this UIA DLL, including generated\n"
        "interfaces, UUID and Wine CRT/import libraries. No UIA implementation changes\n"
        "are omitted. Intersecting shared-header/import additions are unused by this\n"
        "component. It uses the included isolated build recipe, without a claim of\n"
        "byte-identical reproduction of GE's release build.\n"
        "Install only in NEW private staging after checking the exact GE archive and UIA\n"
        "preimage. Replace " + DLL_PATH + "; seal and bind the entire new runtime tree.\n"
        "Do not replace an installed runner or copy a vendor prefix. x86 UIA is unchanged.\n"
        "Unmodified GE/SLR components remain acquired from upstream, with their notices.\n"
        "This component contains no vendor plug-in, licensing or authorization material.\n"
        "Imports resolved by the pinned GE x64 builtin DLL tree:\n" +
        "\n".join(name + ": " + ", ".join(names) for name, names in sorted(imports.items())) + "\n"
    )
    (component / "THIRD_PARTY_NOTICES.txt").write_text(notices)
    artifacts = {role: identity(component / name) for role, name in {
        "test": "uiautomationcore_test.exe", "source": "wine-source.tar.gz",
        "recipe": "build.py", "patch": "uia-null-provider.patch",
        "guard_source": "guard_provider.c", "notices": "THIRD_PARTY_NOTICES.txt",
        "license": "COPYING.LIB"}.items()}
    dll = identity(component / "uiautomationcore.dll")
    manifest = dict(schema=1, base_ge_sha256=GE_SHA, files=[dict(path=DLL_PATH,
        original_sha256=ORIGINAL_DLL_SHA, corrected_sha256=dll["sha256"],
        source=dll["path"], size=dll["size"])], artifacts=artifacts)
    (component / "manifest.json").write_text(json.dumps(manifest, sort_keys=True, indent=2) + "\n")
    artifact = output / "uia-guard.tar.gz"
    with artifact.open("xb") as raw, gzip.GzipFile(fileobj=raw, mode="wb", mtime=0) as gz:
        with tarfile.open(fileobj=gz, mode="w|") as tar:
            for path in sorted(component.iterdir()):
                add_bytes(tar, "uia-guard/" + path.name, path.read_bytes())
    return dict(component=identity(artifact), manifest=manifest, imports=imports)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build", type=Path, required=True)
    parser.add_argument("--ge-archive", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(package(args.build, args.ge_archive, args.output), sort_keys=True))

#!/usr/bin/env python3
"""Seal the corrected Wine MSI modules with their complete corresponding source.

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
ROOT = "msi-reader"
# Component source -> (GE runtime path, GE preimage SHA-256, PE machine).
FILES = {
    "i386-windows/msi.dll": ("files/lib/wine/i386-windows/msi.dll",
        "fa193d0a26c94d1449814c768fab07a73b97677610061a9aca59d9c7ac7b3e8d", 0x014C),
    "x86_64-windows/msi.dll": ("files/lib/wine/x86_64-windows/msi.dll",
        "08a1f048435f1283c47fcc1464b4d4e4d3cba07229737edab11c70af32c6f6c6", 0x8664),
}
# Manifest artifact role -> component file. The runtime composer requires
# exactly these roles.
ROLES = {
    "source": "wine-source.tar.gz", "recipe": "build.py", "build_record": "build-manifest.json",
    "staging_cabinet_patch": build.PATCHES[0][0],
    "upstream_padding_patch": build.PATCHES[1][0],
    "upstream_strref_width_patch": build.PATCHES[2][0],
    "ambiguous_strref_patch": build.PATCHES[3][0],
    "modification_notice_patch": build.PATCHES[4][0],
    "notices": "THIRD_PARTY_NOTICES.txt", "license": "COPYING.LIB",
}


def identity(path, root):
    return dict(path=path.relative_to(root).as_posix(), sha256=build.sha(path),
                size=path.stat().st_size)


def pe_imports(path, machine):
    """Read only PE architecture/import names; reject malformed/out-of-bounds data."""
    data = path.read_bytes()
    if len(data) < 64 or data[:2] != b"MZ":
        raise ValueError("PE header missing")
    off = struct.unpack_from("<I", data, 0x3c)[0]
    if off + 24 > len(data) or data[off:off+4] != b"PE\0\0":
        raise ValueError("PE signature missing")
    found, count = struct.unpack_from("<HH", data, off + 4)
    opt_size = struct.unpack_from("<H", data, off + 20)[0]
    opt = off + 24
    magic, directories = {0x014C: (0x10b, 96), 0x8664: (0x20b, 112)}[machine]
    if found != machine or opt_size < directories + 16 or opt + opt_size + count * 40 > len(data):
        raise ValueError("component must be an intact PE image for its architecture")
    if struct.unpack_from("<H", data, opt)[0] != magic:
        raise ValueError("component PE optional header")
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

    imports, extent = struct.unpack_from("<II", data, opt + directories + 8)
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


def source_archive(output, built, record):
    # Use the pinned pristine source, replacing precisely the changed files.
    # Generated files, object files and diagnostics are never sources.
    prefix = "wine-" + build.WINE + "/"
    changed = {}
    for name, (_, after) in build.SOURCES.items():
        path = built / "wine" / name
        if (build.sha(path) != record["patched_sources"][name]["sha256"]
                or build.git_blob(path) != after):
            raise ValueError("consumed source changed")
        changed[name] = path.read_bytes()
    recipe = built / "build.sh"
    if build.sha(recipe) != record["build_script_sha256"]:
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
            add_bytes(out, "build.sh", recipe.read_bytes())
            add_bytes(out, "SDK.txt", (build.SDK + "\n").encode())


def package(built, ge_archive, output):
    if build.sha(ge_archive) != GE_SHA:
        raise ValueError("GE base archive changed")
    record = json.loads((built / "build-manifest.json").read_text())
    if (record["wine_commit"] != build.WINE or record["sdk_image"] != build.SDK
            or record["wine_archive_sha256"] != build.ARCHIVE_SHA
            or record["patches"] != [dict(file=name, sha256=digest, source=source)
                                     for name, digest, source in build.PATCHES]
            or {name: (row["git_blob_before"], row["git_blob_after"])
                for name, row in record["patched_sources"].items()} != build.SOURCES
            or record["reader_checkpoint"] != {"after": build.READER_PATCH, "path": build.TABLE,
                                               "git_blob": build.READER_BLOB}
            or set(record["artifacts"]) != set(FILES)):
        raise ValueError("build inputs changed")
    imports = {}
    for name, (_, _, machine) in FILES.items():
        path = built / "out" / name
        if build.sha(path) != record["artifacts"][name]["sha256"]:
            raise ValueError("built component changed")
        if not build.check_builtin_pe(path, machine):
            raise ValueError("component is not a Wine builtin PE image")
        imports[name] = pe_imports(path, machine)
    with tarfile.open(ge_archive) as archive:
        members = {m.name: m for m in archive}
        for name, (target, original, _) in FILES.items():
            preimage = archive.extractfile(members[GE_ROOT + "/" + target]).read()
            if hashlib.sha256(preimage).hexdigest() != original:
                raise ValueError("GE MSI preimage changed")
            tree = GE_ROOT + "/" + str(PurePosixPath(target).parent) + "/"
            dlls = {PurePosixPath(m).name.lower() for m in members if m.startswith(tree)}
            if not set(imports[name]).issubset(dlls):
                raise ValueError("component imports unavailable in the exact GE runtime")
    if build.sha(built / "wine.tar.gz") != build.ARCHIVE_SHA:
        raise ValueError("source archive changed")
    with tarfile.open(built / "wine.tar.gz") as wine:
        license_text = wine.extractfile("wine-" + build.WINE + "/COPYING.LIB").read()
    output.mkdir(mode=0o700)
    component = output / ROOT
    component.mkdir(mode=0o700)
    for name in FILES:
        (component / name).parent.mkdir(mode=0o700, exist_ok=True)
        shutil.copyfile(built / "out" / name, component / name)
    inputs = {"build.py": record["builder_sha256"]}
    inputs.update({name: digest for name, digest, _ in build.PATCHES})
    for name, digest in inputs.items():
        if build.sha(built / name) != digest:
            raise ValueError("consumed construction input changed")
        shutil.copyfile(built / name, component / name)
    shutil.copyfile(built / "build-manifest.json", component / "build-manifest.json")
    (component / "COPYING.LIB").write_bytes(license_text)
    source_archive(component / "wine-source.tar.gz", built, record)
    notices = (
        "Wine MSI table-reader correction, 2026-10-15\n"
        "i386-windows/msi.dll, x86_64-windows/msi.dll and Wine source: LGPL-2.1-or-later.\n"
        "The complete patched Wine source is wine-source.tar.gz; original copyright and\n"
        "license notices are retained. Only dlls/msi/table.c and dlls/msi/files.c differ\n"
        "from the pinned Wine source; each carries a dated modification notice. files.c\n"
        "carries wine-staging's msi-cabinet change (Wine bug 50148), which GE-Proton11-7\n"
        "applies. table.c carries upstream Wine commits\n"
        "24bbf46c6f055077df428341595873fe23475a60 (table-stream padding) and\n"
        "e0130972d5ffe578a4a25d75f4c1d0229c880a8c (mismatched string-reference widths),\n"
        "then the Linux-VST-bridge ambiguous string-reference check. The five patch files\n"
        "identify each change in order.\n"
        "COPYING.LIB carries the license. The archive includes build.sh and SDK.txt to\n"
        "compile from that source directly; extract it into a fresh /work volume and run\n"
        "build.sh in the pinned SDK container with XDG_CACHE_HOME=/work/cache.\n"
        "Use two CPUs (cpuset 2,3), 4 GiB memory/no extra swap and 256 PIDs; make uses -j2.\n"
        "build.py and the exact patches also reproduce from the pinned Wine input.\n"
        "Wine commit: " + build.WINE + "\nSDK: " + build.SDK + "\n"
        "GE-Proton11-7 source commit: c191f35dcebbeccfacd3b4c6f6eea026e588c1c2\n"
        "Wine-staging revision: 6cc805ea57132eeaf44764e9213823c9b8d0d300\n"
        "GE applies wine-staging with patchinstall.py --all minus the -W list in\n"
        "patches/protonprep-valve-staging.sh, then its own patches. Every staging patchset\n"
        "GE applies and every GE patch was checked against this build's msi.dll source\n"
        "closure: dlls/msi, the headers and IDL it includes, import-library specs,\n"
        "winecrt0, translations and the widl/winebuild/wrc/winegcc tools. Only\n"
        "staging msi-cabinet changes msi code; it is included. GE's remaining intersecting\n"
        "changes (ntdll-export-PsCreateSystemThread, crypt32-reject-ncrypt-only-private-keys,\n"
        "crypt32-wc3-modern-chain-engine-config, version-GetFileVersionInfoByHandle-stub,\n"
        "kernel32 IsNativeVhdBoot stub, RSX3D configure entry) only add declarations,\n"
        "macros, structures or exports msi does not use, or Unix-only declarations.\n"
        "The modules use the included isolated build recipe, link without a PE timestamp\n"
        "and keep debug information; there is no claim of byte-identical reproduction of\n"
        "GE's release build.\n"
        "Install only in NEW private staging after checking the exact GE archive and both\n"
        "MSI preimages. Replace files/lib/wine/{i386,x86_64}-windows/msi.dll; seal and\n"
        "bind the entire new runtime tree. Do not replace an installed runner or copy a\n"
        "vendor prefix. Unmodified GE/SLR components remain acquired from upstream, with\n"
        "their notices. This component contains no vendor installer, plug-in, licensing\n"
        "or authorization material.\n"
        "Imports resolved by the pinned GE builtin DLL tree of the same architecture:\n" +
        "\n".join(name + ": " + ", ".join(names) for name, names in sorted(imports.items())) + "\n"
    )
    (component / "THIRD_PARTY_NOTICES.txt").write_text(notices)
    artifacts = {role: identity(component / name, component) for role, name in ROLES.items()}
    files = []
    for name, (target, original, _) in FILES.items():
        dll = identity(component / name, component)
        files.append(dict(path=target, original_sha256=original, corrected_sha256=dll["sha256"],
                          source=dll["path"], size=dll["size"]))
    manifest = dict(schema=1, base_ge_sha256=GE_SHA, files=files, artifacts=artifacts)
    (component / "manifest.json").write_text(json.dumps(manifest, sort_keys=True, indent=2) + "\n")
    artifact = output / (ROOT + ".tar.gz")
    with artifact.open("xb") as raw, gzip.GzipFile(fileobj=raw, mode="wb", mtime=0) as gz:
        with tarfile.open(fileobj=gz, mode="w|") as tar:
            for path in sorted(p for p in component.rglob("*") if p.is_file()):
                add_bytes(tar, ROOT + "/" + path.relative_to(component).as_posix(), path.read_bytes())
    return dict(component=identity(artifact, output), manifest=manifest, imports=imports)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build", type=Path, required=True)
    parser.add_argument("--ge-archive", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(package(args.build, args.ge_archive, args.output), sort_keys=True))

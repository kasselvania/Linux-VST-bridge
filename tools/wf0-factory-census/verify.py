#!/usr/bin/env python3
"""Independent PE, manifest, archive, source-surface, and build checks for WA0."""

from __future__ import annotations

import pathlib
import re
import struct
import unicodedata
from typing import Any, Iterable

from common import (
    ARTIFACT_SCHEMA, FAULT_TARGETS, canonical_json, fail, sha256_bytes, sha256_file,
)


SYSTEM_DLLS = {
    "advapi32.dll", "bcrypt.dll", "comctl32.dll", "comdlg32.dll", "crypt32.dll",
    "d2d1.dll", "d3d11.dll", "dwmapi.dll", "dwrite.dll", "dxgi.dll", "gdi32.dll",
    "glu32.dll", "imm32.dll", "kernel32.dll", "msvcrt.dll", "ntdll.dll",
    "ole32.dll", "oleaut32.dll", "opengl32.dll", "rpcrt4.dll", "secur32.dll",
    "setupapi.dll", "shell32.dll",
    "shlwapi.dll", "user32.dll", "ucrtbase.dll", "userenv.dll", "uuid.dll",
    "uxtheme.dll", "version.dll", "winmm.dll", "windowscodecs.dll", "ws2_32.dll",
    "combase.dll",
}
FORBIDDEN_RUNTIME_DLLS = {
    "concrt140.dll", "msvcp140.dll", "msvcp140_1.dll", "msvcp140_2.dll",
    "msvcp140_atomic_wait.dll", "vcruntime140.dll", "vcruntime140_1.dll",
}


def _u16(data: bytes, offset: int) -> int:
    return struct.unpack_from("<H", data, offset)[0]


def _u32(data: bytes, offset: int) -> int:
    return struct.unpack_from("<I", data, offset)[0]


def _cstring(data: bytes, offset: int, limit: int = 4096) -> str:
    end = data.find(b"\0", offset, min(len(data), offset + limit))
    if end < 0:
        fail("PE string is absent or exceeds its bound")
    try:
        return data[offset:end].decode("ascii")
    except UnicodeDecodeError:
        fail("PE import/export name is not ASCII")
    raise AssertionError


def parse_pe(path: pathlib.Path) -> dict[str, Any]:
    data = path.read_bytes()
    if len(data) < 512 or data[:2] != b"MZ":
        fail(f"PE DOS header is absent: {path.name}")
    pe = _u32(data, 0x3C)
    if pe + 24 > len(data) or data[pe:pe + 4] != b"PE\0\0":
        fail(f"PE signature is absent: {path.name}")
    coff = pe + 4
    machine = _u16(data, coff)
    section_count = _u16(data, coff + 2)
    optional_size = _u16(data, coff + 16)
    optional = coff + 20
    if machine != 0x8664 or _u16(data, optional) != 0x20B:
        fail(f"artifact is not PE32+ AMD64: {path.name}")
    if not 1 <= section_count <= 96 or optional + optional_size > len(data):
        fail(f"PE section/optional header is malformed: {path.name}")
    entry_rva = _u32(data, optional + 16)
    directory_count = _u32(data, optional + 108)
    directories = optional + 112
    sections_offset = optional + optional_size
    sections: list[tuple[int, int, int, int]] = []
    for index in range(section_count):
        offset = sections_offset + index * 40
        if offset + 40 > len(data):
            fail("PE section table exceeds file")
        virtual_size = _u32(data, offset + 8)
        virtual_address = _u32(data, offset + 12)
        raw_size = _u32(data, offset + 16)
        raw_offset = _u32(data, offset + 20)
        if raw_offset + raw_size > len(data):
            fail("PE section raw data exceeds file")
        sections.append((virtual_address, max(virtual_size, raw_size), raw_offset, raw_size))

    def rva_offset(rva: int, needed: int = 1) -> int:
        if rva == 0:
            fail("PE directory RVA is null")
        for virtual, span, raw, raw_size in sections:
            if virtual <= rva < virtual + span:
                delta = rva - virtual
                if delta + needed > raw_size:
                    fail("PE RVA points outside section raw data")
                return raw + delta
        fail("PE RVA is outside every section")
        raise AssertionError

    def directory(index: int) -> tuple[int, int]:
        if index >= directory_count or directories + (index + 1) * 8 > optional + optional_size:
            return 0, 0
        return _u32(data, directories + index * 8), _u32(data, directories + index * 8 + 4)

    imports: set[str] = set()
    import_rva, import_size = directory(1)
    if import_rva:
        base = rva_offset(import_rva, min(import_size, 20))
        for index in range(4096):
            offset = base + index * 20
            if offset + 20 > len(data):
                fail("PE import descriptor exceeds file")
            fields = struct.unpack_from("<IIIII", data, offset)
            if fields == (0, 0, 0, 0, 0):
                break
            name_offset = rva_offset(fields[3])
            imports.add(_cstring(data, name_offset).lower())
        else:
            fail("PE import descriptor count exceeds bound")

    delay_rva, delay_size = directory(13)
    if delay_rva:
        base = rva_offset(delay_rva, min(delay_size, 32))
        for index in range(4096):
            offset = base + index * 32
            if offset + 32 > len(data):
                fail("PE delay-import descriptor exceeds file")
            fields = struct.unpack_from("<IIIIIIII", data, offset)
            if fields == (0, 0, 0, 0, 0, 0, 0, 0):
                break
            name_rva = fields[1]
            if fields[0] & 1 == 0:
                fail("PE delay import uses an unsupported VA name")
            imports.add(_cstring(data, rva_offset(name_rva)).lower())
        else:
            fail("PE delay-import descriptor count exceeds bound")

    exports: set[str] = set()
    export_rva, export_size = directory(0)
    if export_rva:
        export_offset = rva_offset(export_rva, min(export_size, 40))
        if export_offset + 40 > len(data):
            fail("PE export directory exceeds file")
        name_count = _u32(data, export_offset + 24)
        names_rva = _u32(data, export_offset + 32)
        if name_count > 4096:
            fail("PE export name count exceeds bound")
        names_offset = rva_offset(names_rva, name_count * 4) if name_count else 0
        for index in range(name_count):
            exports.add(_cstring(data, rva_offset(_u32(data, names_offset + index * 4))))

    return {
        "machine": "AMD64",
        "pe_kind": "PE32+",
        "entry_point_present": entry_rva != 0,
        "imports": sorted(imports),
        "exports": sorted(exports),
        "size": len(data),
        "sha256": sha256_file(path),
    }


def verify_dumpbin_crosscheck(parsed: dict[str, Any], text: str, path: pathlib.Path) -> str:
    normalized = text.lower()
    if "8664 machine (x64)" not in normalized and "machine (x64)" not in normalized:
        fail(f"dumpbin did not identify x64 machine: {path.name}")
    for name in parsed["imports"]:
        if name.lower() not in normalized:
            fail(f"dumpbin omitted parsed import {name}: {path.name}")
    for name in parsed["exports"]:
        if name.lower() not in normalized:
            fail(f"dumpbin omitted parsed export {name}: {path.name}")
    return sha256_bytes(text.encode("utf-8", "strict"))


def verify_pe(path: pathlib.Path, dumpbin_text: str, *,
              required_exports: Iterable[str] = (),
              forbidden_exports: Iterable[str] = (),
              executable: bool = False) -> dict[str, Any]:
    if not path.is_file() or path.is_symlink():
        fail(f"PE artifact is absent or unsafe: {path}")
    parsed = parse_pe(path)
    exports = set(parsed["exports"])
    required = set(required_exports)
    forbidden = set(forbidden_exports)
    if not required <= exports:
        fail(f"required exports missing from {path.name}: {sorted(required - exports)}")
    if exports & forbidden:
        fail(f"forbidden exports present in {path.name}: {sorted(exports & forbidden)}")
    imports = set(parsed["imports"])
    if imports & FORBIDDEN_RUNTIME_DLLS:
        fail(f"dynamic MSVC runtime import is prohibited: {sorted(imports & FORBIDDEN_RUNTIME_DLLS)}")
    unexpected = imports - SYSTEM_DLLS
    if unexpected:
        fail(f"import is outside the closed Windows system roster: {sorted(unexpected)}")
    if executable and not parsed["entry_point_present"]:
        fail(f"scanner/adapter entry point is absent: {path.name}")
    parsed["dumpbin_sha256"] = verify_dumpbin_crosscheck(parsed, dumpbin_text, path)
    return parsed


def artifact_file_records(root: pathlib.Path, roles: dict[str, str]) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    observed: set[str] = set()
    folded: set[str] = set()
    normalized: set[str] = set()
    total_size = 0
    for path in sorted(root.rglob("*"), key=lambda item: item.relative_to(root).as_posix().encode()):
        if path.is_symlink() or (path.exists() and not (path.is_file() or path.is_dir())):
            fail("artifact tree contains an unsafe object")
        if path.is_file():
            relative = path.relative_to(root).as_posix()
            if relative in {"ARTIFACT_MANIFEST.json", "ARTIFACT_MANIFEST.sha256"}:
                continue
            if relative not in roles:
                fail(f"artifact file has no closed role: {relative}")
            if (
                len(relative.encode("utf-8")) > 240
                or "\\" in relative or ":" in relative
                or any(part in {"", ".", ".."} for part in relative.split("/"))
                or relative.casefold() in folded
                or unicodedata.normalize("NFC", relative) in normalized
            ):
                fail(f"artifact path is unsafe or colliding: {relative}")
            if path.stat().st_size > 128 * 1024 * 1024:
                fail(f"artifact file exceeds the single-file bound: {relative}")
            total_size += path.stat().st_size
            if total_size > 512 * 1024 * 1024:
                fail("artifact extracted payload exceeds its bound")
            observed.add(relative)
            folded.add(relative.casefold())
            normalized.add(unicodedata.normalize("NFC", relative))
            records.append({
                "path": relative,
                "type": "file",
                "size": path.stat().st_size,
                "sha256": sha256_file(path),
                "portable_mode": "0444",
                "role": roles[relative],
            })
    if observed != set(roles):
        fail(f"artifact role roster differs: missing={sorted(set(roles) - observed)}")
    if len(records) > 254:
        fail("artifact record count leaves no room for mandatory metadata")
    return records


def artifact_manifest(source_manifest_sha256: str, build_core_sha256: str,
                      records: list[dict[str, Any]]) -> dict[str, Any]:
    paths = [item["path"] for item in records]
    if paths != sorted(paths, key=lambda value: value.encode()) or len(paths) != len(set(paths)):
        fail("artifact records are not uniquely raw-UTF-8 sorted")
    return {
        "schema": ARTIFACT_SCHEMA,
        "implementation_source_manifest_sha256": source_manifest_sha256,
        "build_identity_core_sha256": build_core_sha256,
        "record_count": len(records),
        "records": records,
    }


def complete_tree_records(root: pathlib.Path) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for path in sorted(root.rglob("*"), key=lambda item: item.relative_to(root).as_posix().encode()):
        if path.is_symlink() or (path.exists() and not (path.is_file() or path.is_dir())):
            fail("comparison tree contains an unsafe object")
        if path.is_file():
            records.append({
                "path": path.relative_to(root).as_posix(),
                "size": path.stat().st_size,
                "sha256": sha256_file(path),
            })
    return records


def compare_builds(a: pathlib.Path, b: pathlib.Path) -> dict[str, Any]:
    left = complete_tree_records(a)
    right = complete_tree_records(b)
    if left != right:
        left_map = {item["path"]: item["sha256"] for item in left}
        right_map = {item["path"]: item["sha256"] for item in right}
        differing = sorted(
            path for path in set(left_map) | set(right_map)
            if left_map.get(path) != right_map.get(path)
        )
        fail(f"WF0_BUILD_COMPARISON_BLOCKED: transfer roster differs: {differing}")
    return {
        "level": "byte_identical",
        "path_count": len(left),
        "manifest_sha256": sha256_bytes(canonical_json(left)),
    }


def scanner_component_call_surface(source_root: pathlib.Path, *, ap0: bool = False, ap2: bool = False, ap4: bool = False) -> dict[str, Any]:
    """Require the five inherited plus two WA0 calls and reject audio methods."""
    root = source_root / "windows-factory-probe"
    texts = {
        path.relative_to(root).as_posix(): path.read_text(encoding="utf-8")
        for path in sorted(root.rglob("*"))
        if path.is_file() and path.suffix in {".cpp", ".h"}
    }
    component = texts.get("source/component_instance_session.cpp", "")
    expected = {
        "create_component": r"\bfactory\s*->\s*createInstance\s*\(",
        "get_controller_class_id": r"\bcomponent\s*->\s*getControllerClassId\s*\(",
        "initialize_component": r"\bcomponent\s*->\s*initialize\s*\(",
        "terminate_component": r"\bcomponent\s*->\s*terminate\s*\(",
        "release_component": r"\bcomponent\s*->\s*release\s*\(",
        "query_audio_processor": r"\bcomponent_\s*\.\s*queryInterface\s*\(",
        "release_audio_processor": r"\binterface_\s*->\s*release\s*\(",
    }
    counts = {
        name: len(re.findall(pattern, component))
        for name, pattern in expected.items()
    }
    if counts != {name: 1 for name in expected}:
        fail(f"WA0 plug-in call surface differs: {counts}")
    forbidden_calls = (
        "setIoMode", "getBusCount", "getBusInfo", "getRoutingInfo",
        "activateBus", "setActive", "setState", "getState", "setBusArrangements",
        "getBusArrangement", "canProcessSampleSize", "getLatencySamples",
        "setProcessing", "setupProcessing", "process", "getTailSamples",
        "connect", "disconnect", "notify",
        "createView", "setComponentHandler",
    )
    if ap0:
        permitted = {"getBusCount", "getBusInfo", "getBusArrangement", "canProcessSampleSize",
                     "setBusArrangements", "setupProcessing", "activateBus", "setActive",
                     "setProcessing", "process"}
        forbidden_calls = tuple(value for value in forbidden_calls if value not in permitted)
    if ap2:
        if not ap0:
            fail("AP2 requires the existing offline processing surface")
        forbidden_calls = tuple(value for value in forbidden_calls
                                if value not in {"getLatencySamples", "getTailSamples"})
    if ap4:
        if not ap2:
            fail("AP4 requires the existing hosted processing surface")
        state_source=texts.get("source/mapped_processing.cpp", "")
        for name in ("getState", "setState"):
            pattern=rf"(?:->|\.)\s*{name}\s*\("
            if len(re.findall(pattern,state_source))!=1 or any(re.search(pattern,text) for path,text in texts.items() if path.startswith("source/") and path!="source/mapped_processing.cpp"):
                fail("AP4 state call must occur once in its owner dispatch: "+name)
        forbidden_calls=tuple(value for value in forbidden_calls if value not in {"getState","setState"})
    production = "\n".join(
        text for path, text in texts.items()
        if path.startswith("source/")
    )
    for operation in forbidden_calls:
        if re.search(rf"(?:->|\.)\s*{operation}\s*\(", production):
            fail(f"WA0 scanner calls an out-of-scope plug-in method: {operation}")
    combined = "\n".join(texts.values())
    for interface in ("IEditController", "IConnectionPoint"):
        if interface in combined:
            fail(f"WA0 scanner names an out-of-scope interface: {interface}")
    if (
        "AudioProcessorInterfaceLease" not in combined
        or "42043F99B7DA453CA569E79D9AAEC33D" in combined
        or "audio_processor_ownership_unknown" not in combined
    ):
        fail("WA0 one-owner or eight-state interface boundary differs")
    return {
        "closed_plugin_operation_count": 21 if ap4 else 19 if ap2 else 17 if ap0 else 7,
        "new_audio_interface_operation_count": 2,
        "operation_call_counts": counts,
        "controller_creation_absent": True,
        "audio_processor_method_calls_absent": not ap0,
        "audio_bus_parameter_state_processing_editor_calls_absent": not ap0,
    }


def audio_method_verifier_regression() -> dict[str, Any]:
    forbidden = (
        "setBusArrangements", "getBusArrangement", "canProcessSampleSize",
        "getLatencySamples", "setupProcessing", "setProcessing", "process",
        "getTailSamples",
    )
    pattern = re.compile(
        rf"(?:->|\.)\s*(?:{'|'.join(forbidden)})\s*\("
    )
    rejected = (
        "interface_->process(context)",
        "adjusted . canProcessSampleSize(value)",
        "lease->getTailSamples()",
    )
    accepted = (
        "audio_processor_method_called = false",
        "process_guard()",
        "getTailSamples",
    )
    if any(pattern.search(value) is None for value in rejected):
        fail("receiver-independent audio-method regression missed a call")
    if any(pattern.search(value) is not None for value in accepted):
        fail("receiver-independent audio-method regression rejected non-call text")
    return {
        "mutated_call_samples_rejected": len(rejected),
        "non_call_samples_accepted": len(accepted),
        "receiver_independent": True,
    }


if __name__ == "__main__":
    raise SystemExit("verify.py is a library; use build.py or run.py")

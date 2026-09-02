#!/usr/bin/env python3
"""Strictly normalize the successful raw AGain record into the retained census schema."""

from __future__ import annotations

import json
import re
from typing import Any

from common import canonical_json, fail, sha256_bytes


CENSUS_SCHEMA = "linux-vst-bridge-wf0-factory-census/v1"
TIMELINE_SCHEMA = "linux-vst-bridge-wf0-stage-timeline/v1"
EXPECTED_LIFECYCLE = [
    "scanner_started", "readiness_announced", "supervisor_gate_accepted",
    "module_open_started", "module_opened", "module_entry_succeeded",
    "factory_export_found", "factory_get_started", "factory_obtained",
    "factory_info_obtained", "factory_interface_versions_recorded",
    "class_count_obtained", "class_enumeration_in_progress",
    "class_enumeration_complete", "module_exit_succeeded", "module_unloaded",
    "scanner_completed",
]
EXPECTED_CALLS = [
    "load_library", "init_dll", "get_plugin_factory", "get_factory_info",
    "query_factory_2", "query_factory_3", "count_classes",
    "get_class_info_unicode", "get_class_info_unicode", "get_class_info_unicode",
    "release_factory_3", "release_factory_2", "release_factory_base", "exit_dll",
    "free_library",
]
EXPECTED_CLASSES = [
    ("84E8DE5F92554F5396FAE4133C935A18", "AGain VST3", "Audio Module Class", "Fx", 1),
    ("D39D5B65D7AF42FA843F4AC841EB04F0", "AGain VST3Controller",
     "Component Controller Class", "", 0),
    ("41347FD6FED64094AFBB12B7DBA1D441", "AGain SideChain VST3",
     "Audio Module Class", "Fx", 1),
]
FIELD_CODECS = {"utf8": "utf-8", "utf16le": "utf-16-le"}


def decode_field(hex_value: str, encoding: str, maximum_units: int) -> dict[str, str]:
    if not isinstance(hex_value, str) or not re.fullmatch(r"(?:[0-9a-f]{2})*", hex_value):
        fail("SDK field bytes are not lowercase hexadecimal")
    if encoding not in FIELD_CODECS:
        fail("SDK field encoding label is outside the closed contract")
    raw = bytes.fromhex(hex_value)
    if encoding == "utf16le" and (len(raw) % 2 or len(raw) // 2 >= maximum_units):
        fail("UTF-16 SDK field length is invalid")
    if encoding == "utf8" and len(raw) >= maximum_units:
        fail("UTF-8 SDK field length is invalid")
    try:
        text = raw.decode(FIELD_CODECS[encoding], "strict")
    except UnicodeError:
        fail("SDK field is not strict Unicode")
    if "\x00" in text:
        fail("SDK field contains an embedded NUL")
    return {"text": text, "source_encoding": encoding, "source_bytes_hex": hex_value}


def logical_fuid(raw_hex: str) -> str:
    if not re.fullmatch(r"[0-9A-F]{32}", raw_hex):
        fail("raw TUID is not 16 uppercase bytes")
    raw = bytes.fromhex(raw_hex)
    l1 = int.from_bytes(raw[0:4], "little")
    l2 = (raw[5] << 24) | (raw[4] << 16) | (raw[7] << 8) | raw[6]
    l3 = int.from_bytes(raw[8:12], "big")
    l4 = int.from_bytes(raw[12:16], "big")
    return f"{l1:08X}{l2:08X}{l3:08X}{l4:08X}"


def _flags(value: int, known_bits: dict[int, str]) -> dict[str, Any]:
    unsigned = value & 0xFFFFFFFF
    known = [name for bit, name in sorted(known_bits.items()) if unsigned & bit]
    mask = 0
    for bit in known_bits: mask |= bit
    return {"raw_i32": value, "known": known,
            "unknown_bits_u32_hex": f"{unsigned & ~mask & 0xFFFFFFFF:08x}"}


def sanitized_timeline(run: dict[str, Any]) -> dict[str, Any]:
    timeline = []
    for record in run["records"]:
        if record["event"] == "lifecycle":
            item = {"event": "lifecycle", "sequence": record["sequence"],
                    "state": record["state"]}
            if "class_count" in record: item["class_count"] = record["class_count"]
        else:
            item = {key: record.get(key) for key in (
                "event", "sequence", "attempt_sequence", "operation", "interface",
                "ordinal", "tier", "return_kind") if key in record}
            for key in ("result_u32_hex", "win32_error_u32_hex", "i32_result",
                        "u32_result", "bool_result"):
                if key in record: item[key] = record[key]
        timeline.append(item)
    return {"schema": TIMELINE_SCHEMA, "positive": timeline}


def normalize_positive(run: dict[str, Any], build: dict[str, Any]) -> tuple[dict[str, Any], dict[str, Any]]:
    if (run.get("classification"), run.get("blocker"), run.get("raw_exit"),
        run.get("last_in_flight_operation")) != ("scanner_completed", None, 0, None):
        fail("positive run did not terminate as a complete scanner success")
    records = run["records"]
    lifecycle = [item["state"] for item in records if item.get("event") == "lifecycle"]
    if lifecycle != EXPECTED_LIFECYCLE:
        fail(f"positive lifecycle differs from V5: {lifecycle}")
    starts = [item for item in records if item.get("event") == "call_started"]
    completes = [item for item in records if item.get("event") == "call_completed"]
    if [item["operation"] for item in starts] != EXPECTED_CALLS or len(completes) != 15:
        fail("positive call-attempt ordering/count differs from V5")
    final = records[-1]
    if final.get("state") != "scanner_completed" or final.get("create_instance_called") is not False:
        fail("positive final record is absent or claims class instantiation")
    if final.get("class_count") != 3 or len(final.get("classes", [])) != 3:
        fail("AGain class census is not exactly three records")

    factory = final.get("factory", {})
    vendor = decode_field(factory.get("vendor_hex"), "utf8", 64)
    url = decode_field(factory.get("url_hex"), "utf8", 256)
    email = decode_field(factory.get("email_hex"), "utf8", 128)
    if (vendor["text"], url["text"], email["text"], factory.get("flags_i32")) != (
        "Steinberg Media Technologies", "http://www.steinberg.net",
        "mailto:info@steinberg.de", 16):
        fail("AGain factory metadata differs from the pinned source expectation")

    classes = []
    seen = set()
    for ordinal, raw in enumerate(final["classes"]):
        if raw.get("ordinal") != ordinal:
            fail("AGain class ordinal is not contiguous")
        raw_id = raw.get("raw_tuid_hex")
        if raw_id in seen: fail("duplicate raw TUID")
        seen.add(raw_id)
        class_id = logical_fuid(raw_id)
        category = decode_field(raw.get("category_hex"), "utf8", 32)
        encoding = "utf16le" if raw.get("tier") == "IPluginFactory3.PClassInfoW" else "utf8"
        name = decode_field(raw.get("name_hex"), encoding, 64)
        subcategories = decode_field(raw.get("subcategories_hex"), "utf8", 128)
        vendor_exposed = decode_field(raw.get("vendor_hex"), encoding, 64)
        version = decode_field(raw.get("version_hex"), encoding, 64)
        sdk_version = decode_field(raw.get("sdk_version_hex"), encoding, 64)
        expected = EXPECTED_CLASSES[ordinal]
        if (class_id, name["text"], category["text"], subcategories["text"],
            raw.get("class_flags_u32"), raw.get("cardinality")) != (
            expected[0], expected[1], expected[2], expected[3], expected[4], 2147483647):
            fail(f"AGain class {ordinal} differs from pinned exact order/metadata")
        if version["text"] != "3.8.1.0" or sdk_version["text"] != "VST 3.8.1":
            fail("AGain class version identity differs")
        classes.append({
            "ordinal": ordinal, "class_id": class_id, "class_id_raw_tuid_hex": raw_id,
            "cardinality": raw["cardinality"], "category": category, "name": name,
            "class_info_tier": raw["tier"], "tier_attempts": raw["tier_attempts"],
            "class_flags_if_exposed": _flags(raw["class_flags_u32"], {1: "kDistributable"}),
            "subcategories_if_exposed": subcategories,
            "vendor_if_exposed": vendor_exposed, "version_if_exposed": version,
            "sdk_version_if_exposed": sdk_version,
        })

    timeline = sanitized_timeline(run)
    timeline_sha = sha256_bytes(canonical_json(timeline))
    source = build["implementation_source_manifest"]
    scanner_record = next(item for item in build["artifact_set"]["records"]
                          if item["path"] == "bin/wf0-factory-probe.exe")
    toolchain_sha = sha256_bytes(canonical_json(build["toolchain"]))
    bundle = build["again_bundle_manifest"]
    census = {
        "schema": CENSUS_SCHEMA,
        "scanner_build_identity": {
            "source_manifest_sha256": build["implementation_source_manifest_sha256"],
            "pe_sha256": scanner_record["sha256"], "toolchain_lock_sha256": toolchain_sha,
        },
        "implementation_source_identity": {
            "schema": source["schema"], "commit": source["commit"], "record_count": 26,
            "manifest_sha256": build["implementation_source_manifest_sha256"],
        },
        "reference_fixture_identity": {
            "sdk_root_commit": "3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96",
            "again_entry_blob": "13b920b4b7a74137301bf213cf048e96e82861d4",
            "again_cid_blob": "d32d1640ea187e718baba9cbb039d3a436d53cce",
            "build_manifest_sha256": bundle["sha256"],
        },
        "module_digest": run["protected_snapshot"] and
            next(item["sha256"] for item in bundle["records"]
                 if item["path"] == "Contents/x86_64-win/again.vst3"),
        "module_bundle_identity": {"schema": bundle["schema"], "sha256": bundle["sha256"],
                                   "binary_safe_path": bundle["binary_safe_path"]},
        "dll_search_contract": {"set_default_dll_directories_flags_u32_hex": "00000800",
                                "load_library_ex_flags_u32_hex": "00000900",
                                "absolute_module_path": True, "hfile_null": True},
        "module_entry_presence_and_result": final["module_entry"],
        "factory_interface_support": {
            "IPluginFactory": {"supported": True, "source": "GetPluginFactory"},
            "IPluginFactory2": {"supported": final["factory2_supported"],
                                "query_result_u32_hex": final["factory2_query_u32_hex"]},
            "IPluginFactory3": {"supported": final["factory3_supported"],
                                "query_result_u32_hex": final["factory3_query_u32_hex"]},
        },
        "factory_vendor": vendor, "factory_url": url, "factory_email": email,
        "factory_flags": _flags(factory["flags_i32"], {1: "kClassesDiscardable",
                                                        2: "kLicenseCheck",
                                                        8: "kComponentNonDiscardable",
                                                        16: "kUnicode"}),
        "class_count": 3, "classes": classes,
        "module_exit_presence_and_result": final["module_exit"],
        "module_unload_result": final["module_unload"],
        "call_event_contract": {"closed_operation_count": 15, "started_count": 15,
                                "completed_count": 15, "last_in_flight_operation": None},
        "create_instance_called": False,
        "scanner_exit": {"code_u32_hex": "00000000", "classification": "scanner_completed",
                         "last_stage": "scanner_completed"},
        "stage_timeline_sha256": timeline_sha,
        "explicit_nonclaims": ["no_class_instantiation", "no_audio", "no_gui", "no_bitwig",
                               "no_serum"],
    }
    return census, timeline


if __name__ == "__main__":
    raise SystemExit("normalize.py is a library; run run.py")

#!/usr/bin/env python3
"""Strictly normalize the positive WC0 component session and callback ledger."""

from __future__ import annotations

import json
import re
from typing import Any

from common import canonical_json, fail, sha256_bytes


COMPONENT_SESSION_SCHEMA = "linux-vst-bridge-wc0-component-session/v1"
CALLBACK_LEDGER_SCHEMA = "linux-vst-bridge-wc0-callback-ledger/v1"
TIMELINE_SCHEMA = "linux-vst-bridge-wc0-stage-timeline/v1"
EXPECTED_LIFECYCLE = [
    "scanner_started", "readiness_announced", "supervisor_gate_accepted",
    "module_open_started", "module_opened", "module_entry_succeeded",
    "factory_export_found", "factory_get_started", "factory_obtained",
    "factory_info_obtained", "factory_interface_versions_recorded",
    "class_count_obtained", "class_enumeration_in_progress",
    "class_enumeration_complete", "component_create_in_flight",
    "component_created", "controller_id_in_flight", "controller_id_verified",
    "host_context_ready", "component_initialize_in_flight",
    "component_initialized", "component_terminate_in_flight",
    "component_terminated", "component_release_in_flight", "component_released",
    "object_quiescence_proved", "component_session_closed",
    "module_exit_succeeded", "module_unloaded", "scanner_completed",
]
EXPECTED_CALLS = [
    "load_library", "init_dll", "get_plugin_factory", "get_factory_info",
    "query_factory_2", "query_factory_3", "count_classes",
    "get_class_info_unicode", "get_class_info_unicode", "get_class_info_unicode",
    "create_component", "get_controller_class_id", "initialize_component",
    "terminate_component", "release_component", "release_factory_3",
    "release_factory_2", "release_factory_base", "exit_dll", "free_library",
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
            for key in ("operation", "disposition", "object_quiescence",
                        "component_state", "primary_blocker"):
                if key in record: item[key] = record[key]
        elif record["event"] in {"call_started", "call_completed"}:
            item = {key: record.get(key) for key in (
                "event", "sequence", "attempt_sequence", "operation", "interface",
                "ordinal", "tier", "return_kind") if key in record}
            for key in ("result_u32_hex", "win32_error_u32_hex", "i32_result",
                        "u32_result", "bool_result", "output_nonnull",
                        "host_reference_count", "object_role",
                        "processor_cid_raw_tuid_hex", "requested_iid_raw_tuid_hex",
                        "controller_cid_raw_tuid_hex"):
                if key in record: item[key] = record[key]
        else:
            item = {key: record.get(key) for key in (
                "event", "sequence", "operation", "origin",
                "enclosing_attempt_sequence", "enclosing_operation", "thread_role",
                "reference_count", "result_u32_hex", "output_null",
                "cid_raw_tuid_hex", "iid_raw_tuid_hex") if key in record}
        timeline.append(item)
    return {"schema": TIMELINE_SCHEMA, "positive": timeline}


def normalize_component_positive(
    run: dict[str, Any], build: dict[str, Any]
) -> tuple[dict[str, Any], dict[str, Any], dict[str, Any]]:
    """Normalize the exact AGain processor lifecycle without widening its surface."""
    if (run.get("classification"), run.get("blocker"), run.get("raw_exit"),
        run.get("last_in_flight_operation"), run.get("component_case")) != (
            "scanner_completed", None, 0, None, "exact-again"
        ):
        fail("positive WC0 run did not terminate as one complete exact component session")
    records = run.get("records")
    if not isinstance(records, list) or not records:
        fail("positive WC0 event stream is absent")
    lifecycle = [item["state"] for item in records if item.get("event") == "lifecycle"]
    if lifecycle != EXPECTED_LIFECYCLE:
        fail(f"positive WC0 lifecycle differs: {lifecycle}")
    starts = [item for item in records if item.get("event") == "call_started"]
    completes = [item for item in records if item.get("event") == "call_completed"]
    if [item.get("operation") for item in starts] != EXPECTED_CALLS:
        fail("positive WC0 call-attempt order differs")
    if len(completes) != len(EXPECTED_CALLS):
        fail("positive WC0 call-completion count differs")
    for start, complete in zip(starts, completes):
        if (
            complete.get("attempt_sequence") != start.get("sequence")
            or complete.get("operation") != start.get("operation")
            or complete.get("interface") != start.get("interface")
        ):
            fail("positive WC0 call completion does not pair to its attempt")

    final = records[-1]
    raw_session = final.get("component_session")
    if (
        final.get("event") != "lifecycle"
        or final.get("state") != "scanner_completed"
        or final.get("create_instance_called") is not True
        or final.get("controller_instance_created") is not False
        or not isinstance(raw_session, dict)
    ):
        fail("positive WC0 final record is absent or crosses the claim ceiling")
    expected_processor_raw = "5FDEE8845592534F96FAE4133C935A18"
    expected_component_iid_raw = "31FF31E8D5F20143928EBBEE25697802"
    expected_controller_raw = "655B9DD3AFD7FA42843F4AC841EB04F0"
    create = raw_session.get("create", {})
    controller = raw_session.get("controller_id", {})
    initialize = raw_session.get("initialize", {})
    terminate = raw_session.get("terminate", {})
    release = raw_session.get("release", {})
    host = raw_session.get("host", {})
    callbacks_raw = raw_session.get("callbacks", {})
    if (
        raw_session.get("state") != "component_released"
        or raw_session.get("primary_blocker") is not None
        or raw_session.get("processor_cid_raw_tuid_hex") != expected_processor_raw
        or raw_session.get("requested_iid_raw_tuid_hex") != expected_component_iid_raw
        or create != {
            "attempted": True, "result_u32_hex": "00000000",
            "output_nonnull": True, "tuple_consistent": True,
        }
        or controller != {
            "attempted": True, "buffer_zero_initialized": True,
            "result_u32_hex": "00000000", "raw_tuid_hex": expected_controller_raw,
            "matches_expected": True,
        }
        or initialize != {
            "attempted": True, "result_u32_hex": "00000000", "succeeded": True,
        }
        or terminate != {
            "attempted": True, "returned_ordinary": True,
            "result_u32_hex": "00000000",
        }
        or release != {
            "attempted": True, "returned_ordinary": True, "reference_count": 0,
            "pointer_cleared": True,
        }
        or host != {
            "created": True, "name": "Linux VST Bridge WC0",
            "reference_baseline": 1, "reference_after_initialize": 2,
            "reference_after_terminate": 1,
            "reference_returned_to_baseline": True,
            "owner_release_attempted": True,
            "owner_release_result": 0,
        }
        or raw_session.get("component_call_in_flight") is not False
        or raw_session.get("callback_ledger_closed") is not True
        or raw_session.get("object_quiescence") is not True
        or raw_session.get("inherited_shutdown_permitted") is not True
    ):
        fail("positive AGain component lifecycle or reference sequence differs")

    callback_records = callbacks_raw.get("records")
    expected_callback_shape = [
        ("addRef", "component", "initialize_component", 2),
        ("release", "component", "terminate_component", 1),
        ("release", "owner_local", None, 0),
    ]
    if (
        callbacks_raw.get("capacity") != 64
        or callbacks_raw.get("record_count") != 3
        or callbacks_raw.get("closed") is not True
        or callbacks_raw.get("overflowed") is not False
        or callbacks_raw.get("output_failed") is not False
        or callbacks_raw.get("wrong_thread") is not False
        or callbacks_raw.get("unexpected_object_request") is not False
        or callbacks_raw.get("callback_in_flight") is not False
        or not isinstance(callback_records, list)
        or len(callback_records) != 3
    ):
        fail("positive AGain callback ledger closure differs")
    for raw, expected in zip(callback_records, expected_callback_shape):
        operation, origin, enclosing, count = expected
        if (
            raw.get("operation") != operation
            or raw.get("origin") != origin
            or raw.get("enclosing_operation") != enclosing
            or raw.get("reference_count") != count
            or (origin == "component") != isinstance(
                raw.get("enclosing_attempt_sequence"), int
            )
        ):
            fail("positive AGain callback reference sequence differs")
    callback_events = [item for item in records if item.get("event") == "host_callback"]
    if len(callback_events) != 3:
        fail("positive event stream callback count differs")
    for event, retained in zip(callback_events, callback_records):
        for key in ("operation", "origin", "enclosing_attempt_sequence",
                    "enclosing_operation", "reference_count"):
            if event.get(key) != retained.get(key):
                fail("retained callback ledger differs from synchronously flushed events")

    if final.get("class_count") != 3 or len(final.get("classes", [])) != 3:
        fail("inherited AGain census regression is not the exact three-class roster")
    factory = final.get("factory", {})
    vendor = decode_field(factory.get("vendor_hex"), "utf8", 64)
    url = decode_field(factory.get("url_hex"), "utf8", 256)
    email = decode_field(factory.get("email_hex"), "utf8", 128)
    if (vendor["text"], url["text"], email["text"], factory.get("flags_i32")) != (
        "Steinberg Media Technologies", "http://www.steinberg.net",
        "mailto:info@steinberg.de", 16
    ):
        fail("inherited AGain factory metadata regression differs")
    class_ids = []
    for ordinal, raw in enumerate(final["classes"]):
        raw_id = raw.get("raw_tuid_hex")
        class_id = logical_fuid(raw_id)
        encoding = "utf16le" if raw.get("tier") == "IPluginFactory3.PClassInfoW" else "utf8"
        name = decode_field(raw.get("name_hex"), encoding, 64)["text"]
        category = decode_field(raw.get("category_hex"), "utf8", 32)["text"]
        expected = EXPECTED_CLASSES[ordinal]
        if (class_id, name, category) != expected[:3]:
            fail("inherited AGain ordered class regression differs")
        class_ids.append({
            "ordinal": ordinal, "logical_class_id": class_id,
            "raw_windows_tuid": raw_id, "name": name, "category": category,
        })

    timeline = sanitized_timeline(run)
    callback_ledger = {
        "schema": CALLBACK_LEDGER_SCHEMA,
        "host_name": "Linux VST Bridge WC0",
        "capacity": 64,
        "record_count": 3,
        "records": callback_records,
        "component_originated_sequence": [2, 1],
        "owner_final_release": 0,
        "closed": True,
        "overflowed": False,
        "output_failed": False,
        "wrong_thread": False,
        "unexpected_host_object_request": False,
    }
    source = build["implementation_source_manifest"]
    scanner_record = next(item for item in build["artifact_set"]["records"]
                          if item["path"] == "bin/wf0-factory-probe.exe")
    component_session = {
        "schema": COMPONENT_SESSION_SCHEMA,
        "implementation_source_identity": {
            "schema": source["schema"], "commit": source["commit"],
            "tree": build["source_tree"], "record_count": 19,
            "manifest_sha256": build["implementation_source_manifest_sha256"],
        },
        "scanner_sha256": scanner_record["sha256"],
        "module_sha256": next(
            item["sha256"] for item in build["again_bundle_manifest"]["records"]
            if item["path"] == "Contents/x86_64-win/again.vst3"
        ),
        "processor": {
            "logical_cid": logical_fuid(expected_processor_raw),
            "raw_windows_tuid": expected_processor_raw,
            "requested_interface": "Steinberg::Vst::IComponent",
            "requested_interface_logical_iid": logical_fuid(expected_component_iid_raw),
            "requested_interface_raw_windows_tuid": expected_component_iid_raw,
        },
        "controller": {
            "logical_cid": logical_fuid(expected_controller_raw),
            "raw_windows_tuid": expected_controller_raw,
            "result_u32_hex": controller["result_u32_hex"],
            "queried_before_initialize": True,
            "complete_output_zero_initialized": True,
            "matched": True,
        },
        "create": create,
        "initialize": initialize,
        "terminate": terminate,
        "component_release": release,
        "host_reference_sequence": [1, 2, 1, 0],
        "component_state": "component_released",
        "object_quiescence": {
            "value": True,
            "facts": {
                "initialize_succeeded": True,
                "terminate_attempted_once_and_returned_ordinary": True,
                "component_release_returned_zero": True,
                "component_call_in_flight": False,
                "component_pointer_cleared": True,
                "host_reference_returned_to_baseline": True,
                "host_owner_final_release_returned_zero": True,
                "host_callback_in_flight": False,
                "callback_ledger_closed": True,
            },
        },
        "inherited_wf0_regression": {
            "factory_vendor": vendor["text"],
            "ordered_class_census": class_ids,
            "factory_release_order": [
                "release_factory_3", "release_factory_2", "release_factory_base"
            ],
            "module_exit": final.get("module_exit"),
            "module_unload": final.get("module_unload"),
            "clean_in_process_shutdown": True,
        },
        "call_attribution": {
            "closed_operation_count": 20,
            "new_operation_count": 5,
            "started_count": 20,
            "completed_count": 20,
            "last_in_flight_operation": None,
        },
        "controller_instance_created": False,
        "forbidden_component_method_called": False,
        "explicit_nonclaims": [
            "no_controller_creation", "no_connection_point", "no_audio_processor_census",
            "no_bus_host_call", "no_parameter_host_call", "no_state_host_call",
            "no_processing", "no_audio", "no_events", "no_editor", "no_bitwig",
            "no_serum", "no_proxy", "no_ipc",
        ],
        "stage_timeline_sha256": sha256_bytes(canonical_json(timeline)),
        "callback_ledger_sha256": sha256_bytes(canonical_json(callback_ledger)),
    }
    return component_session, callback_ledger, timeline


if __name__ == "__main__":
    raise SystemExit("normalize.py is a library; run run.py")

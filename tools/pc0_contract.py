"""Shared PC0 acceptance checks; unchanged scalar/call laws from frozen source.

The three product validators and their constants are verbatim extractions from
309b8918c128c0b9e6701d0453dc841a111d5ac5 common.py. They inspect observations;
expected values are never used to create an observation.
"""
from __future__ import annotations
import json
import re


def canonical_json(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":"))+"\n").encode()


def fail(message):
    raise RuntimeError(message)


PC0_CONTRACT_SCHEMA = "linux-vst-bridge-pc0-processing-contract/v1"


PC0_OPERATIONS = (
    "get_bus_count", "get_bus_info", "get_bus_arrangement", "can_process_sample_size",
)


PC0_CALL_COORDINATES = (
    *({"operation": "get_bus_count", "media_type": media, "direction": direction}
      for media in ("kAudio", "kEvent") for direction in ("kInput", "kOutput")),
    *({"operation": "get_bus_info", "media_type": media, "direction": direction,
       "index": 0} for media, direction in (
           ("kAudio", "kInput"), ("kAudio", "kOutput"), ("kEvent", "kInput"))),
    *({"operation": "get_bus_arrangement", "direction": direction, "audio_index": 0}
      for direction in ("kInput", "kOutput")),
    *({"operation": "can_process_sample_size", "symbolic_size": size}
      for size in ("kSample32", "kSample64")),
)


def pc0_expected_contract() -> dict[str, Any]:
    """Pinned-source expectation, never a substitute for an observed contract."""
    return {
        "schema": PC0_CONTRACT_SCHEMA, "lifecycle_state": "Initialized",
        "counts": [dict(item, count=count) for item, count in zip(
            ({key: value for key, value in item.items() if key != "operation"}
             for item in PC0_CALL_COORDINATES[:4]), (1, 1, 1, 0))],
        "buses": [{
            "media_type": media, "direction": direction, "index": 0,
            "name_utf8": name, "channel_count": channels, "bus_type": "kMain",
            "flags_u32_hex": "00000001", "default_active": True, "control_voltage": False,
            "speaker_arrangement": None if media == "kEvent" else {
                "bits_u64_hex": "0000000000000003", "channel_count": 2,
                "recognized_layout": "kStereo"},
        } for media, direction, name, channels in (
            ("kAudio", "kInput", "Stereo In", 2), ("kAudio", "kOutput", "Stereo Out", 2),
            ("kEvent", "kInput", "Event In", 1))],
        "sample_sizes": [{"symbolic_size": size, "tresult_i32": 0,
                          "tresult_u32_hex": "00000000", "supported": True}
                         for size in ("kSample32", "kSample64")],
        "call_count": 11, "complete": True, "mutation_call_count": 0,
    }


def pc0_validate_contract(value: Any, *, exact_again: bool = True) -> dict[str, Any]:
    """Strict bounded named-field admission shared by Deck and renderer."""
    def keys(obj: Any, roster: set[str], label: str) -> None:
        if type(obj) is not dict or set(obj) != roster:
            fail(f"PC0_CONTRACT_INCOMPLETE: {label} keys differ")
    def integer(obj: Any, lower: int, upper: int) -> bool:
        return type(obj) is int and lower <= obj <= upper
    keys(value, set(pc0_expected_contract()), "contract")
    if (value["schema"] != PC0_CONTRACT_SCHEMA or value["lifecycle_state"] != "Initialized"
            or value["complete"] is not True or type(value["mutation_call_count"]) is not int
            or value["mutation_call_count"] != 0):
        fail("PC0_CONTRACT_INCOMPLETE: lifecycle or completion differs")
    counts = value["counts"]
    if type(counts) is not list or len(counts) != 4:
        fail("PC0_BUS_COUNT_BLOCKED: four count records required")
    roster = []
    total = 0
    for count, coordinate in zip(counts, PC0_CALL_COORDINATES[:4]):
        keys(count, {"media_type", "direction", "count"}, "count")
        if any(count[key] != coordinate[key] for key in ("media_type", "direction")):
            fail("PC0_BUS_COUNT_BLOCKED: count coordinates differ")
        if not integer(count["count"], 0, 32) or count["count"] > 64 - total:
            fail("PC0_BUS_COUNT_BLOCKED: count cap/checked sum failed")
        total += count["count"]
    # No implied result roster is constructed until all counts pass.
    for count in counts:
        roster.extend((count["media_type"], count["direction"], index)
                      for index in range(count["count"]))
    if type(value["buses"]) is not list or len(value["buses"]) != total:
        fail("PC0_CONTRACT_INCOMPLETE: bus roster differs")
    audio_count = 0
    for bus, coordinate in zip(value["buses"], roster):
        keys(bus, set(pc0_expected_contract()["buses"][0]), "bus")
        media, direction, index = coordinate
        if ((bus["media_type"], bus["direction"], bus["index"]) != coordinate
                or type(bus["index"]) is not int
                or not integer(bus["channel_count"], 1, 64 if media == "kAudio" else 16)
                or bus["bus_type"] not in {"kMain", "kAux"}
                or type(bus["flags_u32_hex"]) is not str
                or re.fullmatch("[0-9a-f]{8}", bus["flags_u32_hex"]) is None):
            fail("PC0_BUS_INFO_BLOCKED: coordinate or scalar invalid")
        flags = int(bus["flags_u32_hex"], 16)
        if (flags & ~3 or bus["default_active"] is not bool(flags & 1)
                or bus["control_voltage"] is not bool(flags & 2)
                or (media == "kEvent" and flags & 2)):
            fail("PC0_BUS_INFO_BLOCKED: flag projection differs")
        name = bus["name_utf8"]
        try:
            valid_name = (type(name) is str and "\0" not in name
                          and len(name.encode("utf-16-le", "strict")) <= 254
                          and len(name.encode("utf-8", "strict")) <= 508)
        except UnicodeError:
            valid_name = False
        if not valid_name:
            fail("PC0_BUS_INFO_BLOCKED: bounded UTF-16 name invalid")
        arrangement = bus["speaker_arrangement"]
        if media == "kEvent":
            if arrangement is not None:
                fail("PC0_BUS_ARRANGEMENT_BLOCKED: event arrangement forbidden")
        else:
            audio_count += 1
            keys(arrangement, {"bits_u64_hex", "channel_count", "recognized_layout"}, "arrangement")
            bits = arrangement["bits_u64_hex"]
            if type(bits) is not str or re.fullmatch("[0-9a-f]{16}", bits) is None:
                fail("PC0_BUS_ARRANGEMENT_BLOCKED: bitset malformed")
            count = bin(int(bits, 16)).count("1")
            if (not integer(arrangement["channel_count"], 1, 64)
                    or count != arrangement["channel_count"] or count != bus["channel_count"]
                    or arrangement["recognized_layout"] != ("kStereo" if int(bits, 16) == 3 else None)):
                fail("PC0_BUS_ARRANGEMENT_BLOCKED: arrangement projection differs")
    if type(value["sample_sizes"]) is not list or len(value["sample_sizes"]) != 2:
        fail("PC0_SAMPLE_FORMAT_BLOCKED: sample roster differs")
    for sample, size in zip(value["sample_sizes"], ("kSample32", "kSample64")):
        keys(sample, {"symbolic_size", "tresult_i32", "tresult_u32_hex", "supported"}, "sample")
        result = sample["tresult_i32"]
        if (sample["symbolic_size"] != size or not integer(result, 0, 1)
                or sample["tresult_u32_hex"] != f"{result:08x}"
                or sample["supported"] is not (result == 0)):
            fail("PC0_SAMPLE_FORMAT_BLOCKED: sample result projection differs")
    if not integer(value["call_count"], 6, 134) or value["call_count"] != 6 + total + audio_count:
        fail("PC0_CONTRACT_INCOMPLETE: call count differs")
    if exact_again and canonical_json(value) != canonical_json(pc0_expected_contract()):
        fail("PC0_CONTRACT_INCOMPLETE: exact AGain pre-setup contract differs")
    return value


def pc0_validate_call_facts(facts: dict[str, Any]) -> None:
    expected = [
        "load_library", "init_dll", "get_plugin_factory", "get_factory_info",
        "query_factory_2", "query_factory_3", "count_classes",
        "get_class_info_unicode", "get_class_info_unicode", "get_class_info_unicode",
        "create_component", "get_controller_class_id", "initialize_component",
        "query_audio_processor", *[item["operation"] for item in PC0_CALL_COORDINATES],
        "release_audio_processor", "terminate_component", "release_component",
        "release_factory_3", "release_factory_2", "release_factory_base", "exit_dll", "free_library",
    ]
    counts = facts.get("pc0_operation_counts")
    if canonical_json(counts) != canonical_json(dict(zip(PC0_OPERATIONS, (4, 3, 2, 2)))):
        fail("PC0_CONTRACT_INCOMPLETE: operation counts differ")
    ledger = facts.get("ledger")
    if type(ledger) is not list or len(ledger) != 66:
        fail("PC0_CONTRACT_INCOMPLETE: full paired call ledger absent")
    inherited = [
        (None, {}, {"return_kind": "handle_nonnull", "win32_error_u32_hex": "00000000"}),
        (None, {}, {"return_kind": "bool", "bool_result": True}),
        (None, {}, {"return_kind": "pointer_nonnull"}),
        ("IPluginFactory", {}, {"return_kind": "tresult", "result_u32_hex": "00000000"}),
        ("IPluginFactory", {}, {"return_kind": "tresult", "result_u32_hex": "00000000"}),
        ("IPluginFactory", {}, {"return_kind": "tresult", "result_u32_hex": "00000000"}),
        ("IPluginFactory", {}, {"return_kind": "i32", "i32_result": 3}),
        ("IPluginFactory3", {"ordinal": 0, "tier": "factory_3_unicode"},
         {"return_kind": "tresult", "result_u32_hex": "00000000"}),
        ("IPluginFactory3", {"ordinal": 1, "tier": "factory_3_unicode"},
         {"return_kind": "tresult", "result_u32_hex": "00000000"}),
        ("IPluginFactory3", {"ordinal": 2, "tier": "factory_3_unicode"},
         {"return_kind": "tresult", "result_u32_hex": "00000000"}),
        ("IPluginFactory", {
            "object_role": "again_processor_component",
            "processor_cid_raw_tuid_hex": "5FDEE8845592534F96FAE4133C935A18",
            "requested_iid_raw_tuid_hex": "31FF31E8D5F20143928EBBEE25697802",
        }, {"return_kind": "tresult", "result_u32_hex": "00000000",
            "output_nonnull": True, "object_role": "again_processor_component"}),
        ("IComponent", {"object_role": "again_processor_component"}, {
            "return_kind": "tresult", "result_u32_hex": "00000000",
            "controller_cid_raw_tuid_hex": "655B9DD3AFD7FA42843F4AC841EB04F0",
            "object_role": "again_processor_component",
        }),
        ("IComponent", {"object_role": "again_processor_component"}, {
            "return_kind": "tresult", "result_u32_hex": "00000000",
            "host_reference_count": 2, "object_role": "again_processor_component",
        }),
        ("IComponent", {
            "object_role": "again_processor_component",
            "requested_interface": "Steinberg::Vst::IAudioProcessor",
            "requested_iid_raw_tuid_hex": "993F0442DAB73C45A569E79D9AAEC33D",
        }, {"return_kind": "tresult", "result_u32_hex": "00000000",
            "output_nonnull": True, "object_role": "again_processor_component",
            "requested_interface": "Steinberg::Vst::IAudioProcessor"}),
        ("IAudioProcessor", {"object_role": "again_audio_processor_interface"}, {
            "return_kind": "reference_count", "u32_result": 1,
            "object_role": "again_audio_processor_interface",
        }),
        ("IComponent", {"object_role": "again_processor_component"}, {
            "return_kind": "tresult", "result_u32_hex": "00000000",
            "host_reference_count": 1, "object_role": "again_processor_component",
        }),
        ("IComponent", {"object_role": "again_processor_component"}, {
            "return_kind": "reference_count", "u32_result": 0,
            "object_role": "again_processor_component",
        }),
        ("IPluginFactory3", {}, {"return_kind": "u32", "u32_result": 2}),
        ("IPluginFactory2", {}, {"return_kind": "u32", "u32_result": 1}),
        ("IPluginFactory", {}, {"return_kind": "u32", "u32_result": 0}),
        (None, {}, {"return_kind": "bool", "bool_result": True}),
        (None, {}, {"return_kind": "bool_true", "win32_error_u32_hex": "00000000"}),
    ]
    # PC0's eleven calls are inserted after the inherited query and before the
    # inherited audio-interface release.  These positions map the closed
    # 33-call ledger back to the 22-call accepted WC0/WA0 ledger.
    inherited_indexes = [*range(14), *range(25, 33)]
    inherited_by_full_index = dict(zip(inherited_indexes, inherited))
    previous = 0
    new_index = 0
    for index, operation in enumerate(expected):
        start, end = ledger[index * 2:index * 2 + 2]
        for record in (start, end):
            if (type(record) is not dict
                    or type(record.get("sequence")) is not int
                    or not previous < record["sequence"] <= 2048
                    or record.get("operation") != operation):
                fail("PC0_CONTRACT_INCOMPLETE: call record order or shape differs")
            previous = record["sequence"]
        if (start.get("event") != "call_started" or end.get("event") != "call_completed"
                or type(end.get("attempt_sequence")) is not int
                or end["attempt_sequence"] != start["sequence"]
                or any(end.get(key) != start.get(key) for key in ("interface", "ordinal", "tier"))):
            fail("PC0_CONTRACT_INCOMPLETE: call pairing differs")
        common_start = {"event": "call_started", "sequence": start["sequence"],
                        "operation": operation, "interface": start.get("interface"),
                        "ordinal": start.get("ordinal"), "tier": start.get("tier")}
        common_end = {"event": "call_completed", "sequence": end["sequence"],
                      "attempt_sequence": start["sequence"], "operation": operation,
                      "interface": end.get("interface"), "ordinal": end.get("ordinal"),
                      "tier": end.get("tier")}
        if operation in PC0_OPERATIONS:
            coordinate = PC0_CALL_COORDINATES[new_index]
            fields = {key: item for key, item in coordinate.items() if key != "operation"}
            interface = "IComponent" if operation in {"get_bus_count", "get_bus_info"} else "IAudioProcessor"
            expected_start = dict(common_start, interface=interface, **fields)
            expected_end = dict(common_end, interface=interface, **fields,
                                return_kind="int32" if operation == "get_bus_count" else "tresult")
            if end["sequence"] != start["sequence"] + 1:
                fail("PC0_CONTRACT_INCOMPLETE: completion was not immediate")
            if new_index < 4:
                expected_end["i32_result"] = (1, 1, 1, 0)[new_index]
            else:
                expected_end["result_u32_hex"] = "00000000"
            if start != expected_start or end != expected_end:
                fail("PC0_CONTRACT_INCOMPLETE: fixed PC0 call fact differs")
            new_index += 1
        else:
            interface, start_fields, end_fields = inherited_by_full_index[index]
            expected_start = dict(common_start, interface=interface, **start_fields)
            expected_end = dict(common_end, interface=interface, **end_fields)
            if start != expected_start or end != expected_end:
                fail("PC0_CONTRACT_INCOMPLETE: inherited positive call fact differs")


EXPECTED_LIFECYCLE = [
    "scanner_started", "readiness_announced", "supervisor_gate_accepted",
    "module_open_started", "module_opened", "module_entry_succeeded",
    "factory_export_found", "factory_get_started", "factory_obtained",
    "factory_info_obtained", "factory_interface_versions_recorded",
    "class_count_obtained", "class_enumeration_in_progress",
    "class_enumeration_complete", "component_create_in_flight",
    "component_created", "controller_id_in_flight", "controller_id_verified",
    "host_context_ready", "component_initialize_in_flight",
    "component_initialized", "audio_processor_query_in_flight",
    "audio_processor_lease_acquired", "audio_processor_release_in_flight",
    "audio_processor_lease_retired", "audio_interface_quiescence_proved",
    "component_terminate_in_flight",
    "component_terminated", "component_release_in_flight", "component_released",
    "object_quiescence_proved", "component_session_closed",
    "module_exit_succeeded", "module_unloaded", "scanner_completed",
]


SHUTDOWN_OPERATIONS = (
    'terminate_component', 'release_component', 'release_factory_3',
    'release_factory_2', 'release_factory_base', 'exit_dll', 'free_library',
)


def acceptance_facts(observed, audio, component, timeline):
    calls = [{'interface':None, 'ordinal':None, 'tier':None, **record}
             for record in timeline if record.get('event') in {'call_started', 'call_completed'}]
    # No expected census value supplies a result: all fields below are observed
    # or normalized from the actual stream, and then independently admitted.
    counts = {op: sum(r['event'] == 'call_started' and r['operation'] == op for r in calls)
              for op in PC0_OPERATIONS}
    return {
        'call_facts': {'pc0_operation_counts': counts, 'ledger': calls},
        'lifecycle': [r['state'] for r in timeline if r.get('event') == 'lifecycle'],
        'audio_lease': {key: audio[key] for key in ('state', 'query', 'lease_acquired',
            'release', 'pointer_cleared', 'call_in_flight', 'callback_ledger_unchanged',
            'audio_interface_quiescence')},
        'component_quiescence': component['object_quiescence'],
        'host_reference_sequence': component['host_reference_sequence'],
        'containment': observed['cleanup'],
        'last_in_flight_operation': observed['last_in_flight_operation'],
        'last_lifecycle': observed['last_lifecycle'],
        'stdout_sha256': observed['stdout_sha256'], 'stderr_sha256': observed['stderr_sha256'],
        'stderr_bytes': observed['stderr_bytes'],
    }


def validate_acceptance_summary(summary):
    if (type(summary) is not dict or set(summary) != {
            'run_id', 'processing_contract', 'shutdown', 'raw_exit', 'verification'}
            or re.fullmatch('[0-9a-f]{32}', str(summary['run_id'])) is None
            or type(summary['raw_exit']) is not int or summary['raw_exit'] != 0):
        fail('PC0 acceptance summary is not a complete successful observation')
    pc0_validate_contract(summary['processing_contract'])
    facts = summary['verification']
    if type(facts) is not dict or set(facts) != {
            'call_facts', 'lifecycle', 'audio_lease', 'component_quiescence',
            'host_reference_sequence', 'containment', 'last_in_flight_operation',
            'last_lifecycle', 'stdout_sha256', 'stderr_sha256', 'stderr_bytes',
            'stage_absent', 'protected_before_sha256', 'protected_after_sha256'}:
        fail('PC0 acceptance verification facts differ')
    pc0_validate_call_facts(facts['call_facts'])
    shutdown = {'operations': {op: {'disposition':'completed', 'source':'scanner_call_ledger'}
                              for op in SHUTDOWN_OPERATIONS},
                'clean_in_process_shutdown':True, 'physical_containment_only':False}
    if canonical_json(summary['shutdown']) != canonical_json(shutdown):
        fail('PC0 acceptance inherited shutdown incomplete')
    lease = facts['audio_lease']
    if canonical_json(lease) != canonical_json({
            'state':'audio_processor_lease_retired',
            'query':{'output_zero_initialized':True,'attempted':True,'result_u32_hex':'00000000',
                     'output_nonnull':True,'tuple_consistent':True},
            'lease_acquired':True,'release':{'attempted':True,'returned_ordinary':True,'reference_count':1},
            'pointer_cleared':True,'call_in_flight':False,'callback_ledger_unchanged':True,
            'audio_interface_quiescence':True}):
        fail('PC0 acceptance interface lease differs')
    q = facts['component_quiescence']
    required = {'audio_interface_quiescent_before_terminate', 'initialize_succeeded',
                'terminate_attempted_once_and_returned_ordinary', 'component_release_returned_zero',
                'component_call_in_flight', 'component_pointer_cleared',
                'host_reference_returned_to_baseline', 'host_owner_final_release_returned_zero',
                'host_callback_in_flight', 'callback_ledger_closed'}
    if (type(q) is not dict or set(q) != {'value','facts'} or q['value'] is not True
            or type(q['facts']) is not dict or set(q['facts']) != required
            or any(value is not (key not in {'component_call_in_flight','host_callback_in_flight'})
                   for key,value in q['facts'].items())
            or canonical_json(facts['host_reference_sequence']) != canonical_json([1,2,1,0])):
        fail('PC0 acceptance component quiescence differs')
    if (canonical_json(facts['containment']) != canonical_json({
            'owned_descendants_zero':True,'process_group_empty':True})
            or facts['stage_absent'] is not True or facts['last_in_flight_operation'] is not None
            or facts['last_lifecycle'] != 'scanner_completed'):
        fail('PC0 acceptance containment or completion differs')
    for key in ('stdout_sha256','stderr_sha256','protected_before_sha256','protected_after_sha256'):
        if type(facts[key]) is not str or re.fullmatch('[0-9a-f]{64}',facts[key]) is None:
            fail('PC0 acceptance stream/protected digest differs')
    if (facts['protected_before_sha256'] != facts['protected_after_sha256']
            or type(facts['stderr_bytes']) is not int or not 0 <= facts['stderr_bytes'] <= 65536):
        fail('PC0 acceptance protected state or stream bound differs')
    lifecycle = list(EXPECTED_LIFECYCLE)
    position = lifecycle.index('audio_processor_release_in_flight')
    lifecycle[position:position] = ['bus_counts_validated','bus_info_complete',
        'speaker_arrangements_complete','pre_setup_contract_complete']
    if facts['lifecycle'] != lifecycle:
        fail('PC0 acceptance lifecycle order differs')
    return summary


def acceptance_execution_input_sha256(binding, runtime):
    import hashlib
    return hashlib.sha256(canonical_json({
        'schema':'linux-vst-bridge-pc0-acceptance-execution-input/v1',
        'binding':binding,
        'observed_runtime_sha256':runtime['launch_critical_manifest_sha256'],
        'declared_runtime_inputs_sha256':runtime['declared_inputs_sha256'],
    })).hexdigest()

#!/usr/bin/env python3
"""Private reservation worker; pinned PC0 primitives, never the acceptance driver.

Delivered as verified stdin bytes. Only the classified adapter supplies requests;
this file is not a campaign authority or a general purpose execution CLI.
"""
from __future__ import annotations

import hashlib
import json
import os
import pathlib
import re
import sys
import types

_SUPPORT_SOURCES = {}

MAX_BYTES = 2 * 1024 * 1024
SCHEMA = "linux-vst-bridge-pc0-reservation-diagnostic/v2"
CLASS = "DIAGNOSTIC_NON_AUTHORITATIVE"


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def read(path, maximum=MAX_BYTES):
    if path.is_symlink() or not path.is_file() or not 0 < path.stat().st_size <= maximum:
        raise RuntimeError("unsafe diagnostic file")
    raw = path.read_bytes()
    if len(raw) > maximum:
        raise RuntimeError("diagnostic exceeds bound")
    return raw


def object_file(path):
    raw = read(path)
    value = json.loads(raw)
    if not isinstance(value, dict) or canonical(value) != raw:
        raise RuntimeError("noncanonical diagnostic")
    return value


def namespace(proof, binding):
    if binding["execution_class"] != CLASS or binding["acceptance_eligible"] is not False:
        raise RuntimeError("diagnostic class required")
    for key in ("campaign_identity", "reservation_identity", "worker_sha256"):
        if re.fullmatch(r"[0-9a-f]{64}", str(binding[key])) is None:
            raise RuntimeError("invalid diagnostic identity")
    root = proof / "diagnostics" / CLASS / binding["campaign_identity"] / binding["reservation_identity"]
    for path in (root, *root.parents):
        if path.is_symlink():
            raise RuntimeError("unsafe diagnostic namespace")
    return root


def inspect(proof, binding):
    """Bounded retrieval has no environment, process, runner or fixture probes."""
    root = namespace(proof, binding)
    if not root.exists():
        return {"state": "absent"}
    if not root.is_dir():
        raise RuntimeError("unsafe diagnostic root")
    intent = root / "intent.json"
    if not intent.exists():
        return {"state": "unknown"}
    if object_file(intent) != binding:
        raise RuntimeError("reservation intent differs")
    path = root / "observation.json"
    sidecar = root / "observation.sha256"
    if not path.exists() or not sidecar.exists():
        return {"state": "unknown"}
    raw = read(path)
    if read(sidecar, 256) != f"{digest(raw)}  observation.json\n".encode():
        raise RuntimeError("diagnostic sidecar differs")
    value = object_file(path)
    if value.get("schema") != SCHEMA or value.get("binding") != binding:
        raise RuntimeError("reservation observation differs")
    return {"state": "observed", "observation": value}


def publish(path, raw):
    if len(raw) > MAX_BYTES:
        raise RuntimeError("diagnostic exceeds bound")
    # Lock ownership is already held. Never replace an existing publication.
    with path.open("xb") as stream:
        stream.write(raw)
        stream.flush()
        os.fsync(stream.fileno())
    sync(path.parent)


def sync(path):
    fd = os.open(path, os.O_RDONLY)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def primitives():
    sys.path.insert(0, str(pathlib.Path.cwd() / "tools/wf0-factory-census"))
    for name, source in _SUPPORT_SOURCES.items():
        source_digest = digest(source.encode())
        if name in sys.modules:
            if getattr(sys.modules[name], "_pc0_source_sha256", None) != source_digest:
                raise RuntimeError("diagnostic helper module identity differs")
            continue
        module = types.ModuleType(name)
        module.__file__ = "<" + name + ">"
        module._pc0_source_sha256 = source_digest
        sys.modules[name] = module
        exec(compile(source, module.__file__, "exec"), module.__dict__)
    import common
    import artifacts
    import run
    return common, artifacts, run


def preflight(binding):
    c, a, r = primitives()
    source = r.pc0_v3_require_frozen_source(binding["execution_source"]["commit"], detached=True)
    if c.dx0_source_role(source) != binding["execution_source"]:
        raise RuntimeError("stopped source differs")
    handoff = r.pc0_v3_verify_source_handoff(
        c.dx0_deck_source_parent() / source["commit"], source["commit"], reconstruct=False)
    c.process_guard()  # Includes this process; code must never be sent in argv.
    c.deck_fixture_identity()
    from pc0_diagnostic_runtime import verify_diagnostic_runner
    runner = verify_diagnostic_runner()
    if runner["baseline_contract_sha256"] != binding["runtime_identity"]:
        raise RuntimeError("runtime contract differs")
    before = c.protected_snapshot()
    host_root = c.dx0_deck_host_artifact_parent() / binding["host_manifest_sha256"]
    build_input = a.read_canonical_json(host_root / "DX0_WINDOWS_HOST_BUILD_RECEIPT.json")["windows_build_input"]["sha256"]
    host = a.verify_host_store(host_root, build_input)
    fixture = a.verify_fixture_store(c.dx0_deck_fixture_parent() / binding["fixture_bundle_sha256"])
    if (build_input != binding["windows_build_input_identity"]
            or host["manifest_sha256"] != binding["host_manifest_sha256"]
            or host["build_receipt"]["producer_source"] != binding["producer_source"]
            or host["build_receipt"]["workflow"]["run_id"] != binding["producer_run_id"]
            or host["build_receipt"]["workflow"]["run_attempt"] != binding["producer_run_attempt"]
            or host["custody"]["artifact"]["id"] != binding["artifact_id"]
            or fixture["identity_sha256"] != binding["fixture_identity"]
            or c.DX0_AGAIN_MODULE_SHA256 != binding["fixture_module_sha256"]):
        raise RuntimeError("frozen artifact or fixture differs")
    plan = c.dx0_closed_plan(c.PC0_PLAN_ID)
    if c.dx0_identity_sha256(plan) != binding["legacy_plan_sha256"]:
        raise RuntimeError("closed plan differs")
    deck_input = c.dx0_deck_execution_input(source["commit"], host["manifest_sha256"], fixture["identity_sha256"], binding["legacy_plan_sha256"])
    diagnostic_input = {"schema": "linux-vst-bridge-pc0-diagnostic-execution-input/v1",
                        "stopped_contract_input_sha256": c.dx0_identity_sha256(deck_input),
                        "observed_runtime_sha256": runner["launch_critical_manifest_sha256"],
                        "declared_runtime_inputs_sha256": runner["declared_inputs_sha256"]}
    return c, r, source, host, fixture, before, digest(canonical(diagnostic_input)), handoff, runner


def failure_data(r, observed, binding, deck_sha, retirement_disposition):
    """Build the pinned bounded failure projection inside our sole publication."""
    timeline = r.sanitized_timeline(observed)["positive"]
    shutdown = observed["inherited_shutdown"]
    if shutdown.get("operations") == {}:
        started = {v.get("operation") for v in timeline if v.get("event") == "call_started"}
        completed = {v.get("operation") for v in timeline if v.get("event") == "call_completed"}
        shutdown = {**shutdown, "operations": {
            op: {"disposition": ("completed" if op in completed else
                 "attempted_without_ordinary_return" if op in started else "not_attempted_prior_stage"),
                 "source": "scanner_call_ledger" if op in started else "supervisor_call_ledger_absence"}
            for op in r.PC0_SHUTDOWN_OPERATIONS}}
    value = {key: observed[key] for key in (
        "run_id", "raw_exit", "classification", "secondary_cleanup_blocker",
        "last_lifecycle", "last_in_flight_operation", "call_counts",
        "audio_processor_observer_state", "audio_interface_quiescence", "cleanup",
        "stdout_sha256", "stderr_sha256", "stderr_bytes")}
    value.update({
        "schema": r.PC0_FAILURE_DIAGNOSTIC_SCHEMA,
        "operation_nonce": binding["reservation_identity"][:32],
        "phase_nonce": binding["reservation_identity"][32:],
        "execution_source": binding["execution_source"],
        "execution_input_sha256": deck_sha, "proof_plan_sha256": binding["legacy_plan_sha256"],
        "primary_blocker": observed["blocker"], "durable_record_count": len(timeline),
        "durable_records": timeline, "inherited_shutdown": shutdown,
        "environment_retirement_disposition": retirement_disposition,
        "protected_snapshot_sha256": digest(canonical(observed["protected_snapshot"])),
        "runner_identity_sha256": observed["runner_identity"]["launch_critical_manifest_sha256"]})
    from pc0_diagnostic_primitives import validate_failure_diagnostic
    validate_failure_diagnostic(
        value, expected_source=binding["execution_source"],
        expected_execution_input_sha256=deck_sha, expected_plan_sha256=binding["legacy_plan_sha256"],
        expected_operation_nonce=binding["reservation_identity"][:32],
        expected_phase_nonce=binding["reservation_identity"][32:],
        expected_runner_identity_sha256=value["runner_identity_sha256"])
    return value


def execute(proof, binding):
    retained = inspect(proof, binding)
    if retained["state"] != "absent":
        return {**retained, "effects": {"deck_workloads": 0, "diagnostic_publications": 0}}
    c, r, source, host, fixture, before, deck_sha, _handoff, runtime = preflight(binding)
    import pc0_diagnostic_primitives as diagnostic
    root = namespace(proof, binding)
    root.parent.mkdir(parents=True, exist_ok=True)
    try:
        root.mkdir(mode=0o700)
    except FileExistsError:
        return {"state": "unknown"}
    sync(root.parent)
    publish(root / "intent.json", canonical(binding))
    (root / "lock").mkdir(mode=0o700)
    sync(root)
    # Once the durable claim exists, every incomplete path stays unknown. No
    # process or acknowledgement failure permits another launch here.
    environment = None
    observed = None
    retired = False
    retirement_disposition = "not_attempted_process_containment_unproved"
    summary = None
    kind = "INCONCLUSIVE"
    classification = "PC0_DIAGNOSTIC_INCONCLUSIVE"
    try:
        c.process_guard()
        environment = diagnostic.create_dx0_environment(
            binding["reservation_identity"][:32], host=host, fixture=fixture,
            execution_source=binding["execution_source"], deck_execution_input_sha256=deck_sha,
            runner_identity_sha256=runtime["launch_critical_manifest_sha256"])
        observed = diagnostic.supervise(environment, mode=c.PC0_MODE)
        contained = observed.get("cleanup") == {"owned_descendants_zero": True, "process_group_empty": True}
        if not contained:
            environment = None  # Do not remove files beneath potentially live children.
        if observed["classification"] == "scanner_completed" and contained:
            audio, _component, _timeline = r.normalize_wa0_positive(
                observed, r._build_for_normalizer(source, host, fixture), pre_setup=True)
            summary = {"run_id": observed["run_id"], "processing_contract": audio["processing_contract"]}
            kind, classification = "SUCCESS", "PC0_DIAGNOSTIC_OBSERVED"
        else:
            # Keep the original acceptance result and lock untouched.
            classification = "PC0_DIAGNOSTIC_FAILED"
            kind = "FAILED"
    except Exception:
        # Raw exceptions can contain private paths. Retain only bounded facts.
        classification = "PC0_DIAGNOSTIC_INCONCLUSIVE"
    finally:
        if environment is not None and observed is not None:
            try:
                retirement = diagnostic.retire_environment(environment,
                    runner_identity_sha256=runtime["launch_critical_manifest_sha256"])
                retired = retirement["environment_retired"] is True and retirement["stage_absent"] is True
                retirement_disposition = "retired" if retired else "failed"
            except Exception:
                retired = False
                retirement_disposition = "failed"
    if observed is None:
        return {"state": "unknown", "effects": {"deck_workloads": "unknown", "diagnostic_publications": 0}}
    protected = "UNKNOWN"
    try:
        protected = "UNCHANGED" if c.protected_snapshot() == before else "CHANGED"
    except Exception:
        pass
    cleanup = "INCOMPLETE"
    if retired and observed.get("cleanup") == {"owned_descendants_zero": True, "process_group_empty": True}:
        try:
            c.process_guard()
            cleanup = "COMPLETE"
        except Exception:
            pass
    if summary is None:
        try:
            summary = {"failure": failure_data(r, observed, binding, deck_sha, retirement_disposition),
                       "supervision_error": observed.get("supervision_error")}
        except Exception:
            return {"state": "unknown", "effects": {"deck_workloads": 1, "diagnostic_publications": 0}}
    if cleanup != "COMPLETE" or protected != "UNCHANGED":
        kind, classification = "INCONCLUSIVE", "PC0_DIAGNOSTIC_INCONCLUSIVE"
    value = {"schema": SCHEMA, "binding": binding, "execution_input_sha256": deck_sha,
             "kind": kind, "classification": classification, "summary": summary,
             "cleanup": cleanup, "protected": protected, "runtime_observation": runtime}
    raw = canonical(value)
    publish(root / "observation.json", raw)
    publish(root / "observation.sha256", f"{digest(raw)}  observation.json\n".encode())
    return {"state": "observed", "observation": value,
            "effects": {"deck_workloads": 1, "diagnostic_publications": 1}}


def main():
    action, encoded = sys.argv[2:4]
    binding = json.loads(bytes.fromhex(encoded))
    if binding["worker_sha256"] != sys.argv[1]:
        raise RuntimeError("worker identity differs")
    proof = pathlib.Path.home() / ".local/share/linux-vst-bridge/proof"
    if action == "inspect":
        value = inspect(proof, binding)
    elif action == "preflight":
        preflight(binding)
        value = {"state": "ready"}
    elif action == "execute":
        value = execute(proof, binding)
    else:
        raise RuntimeError("unsupported diagnostic action")
    raw = canonical(value)
    if len(raw) > MAX_BYTES:
        raise RuntimeError("diagnostic reply exceeds bound")
    sys.stdout.buffer.write(raw)


if __name__ == "__main__":
    main()

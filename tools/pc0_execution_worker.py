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
import tempfile

_SUPPORT_SOURCES = {}

MAX_BYTES = 2 * 1024 * 1024
SCHEMA = "linux-vst-bridge-pc0-reservation-diagnostic/v2"
CLASS = "DIAGNOSTIC_NON_AUTHORITATIVE"
ACCEPTANCE = "ACCEPTANCE_CANDIDATE"


def schema_for(binding):
    if binding.get("product") in {"AP0", "AP1"}:
        return "linux-vst-bridge-" + binding["product"].lower() + "-reservation/v1"
    return "linux-vst-bridge-pc0-reservation-acceptance/v1" if binding["execution_class"] == ACCEPTANCE else SCHEMA


def is_acceptance(binding):
    return binding["execution_class"] == ACCEPTANCE


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
    if (binding["execution_class"] not in {CLASS, ACCEPTANCE}
            or binding["acceptance_eligible"] is not is_acceptance(binding)):
        raise RuntimeError("diagnostic class required")
    identity_key = "candidate_identity" if is_acceptance(binding) else "campaign_identity"
    for key in (identity_key, "reservation_identity", "worker_sha256"):
        if re.fullmatch(r"[0-9a-f]{64}", str(binding[key])) is None:
            raise RuntimeError("invalid diagnostic identity")
    root = proof / ("acceptance" if is_acceptance(binding) else "diagnostics") / binding["execution_class"] / binding[identity_key] / binding["reservation_identity"]
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
        checkpoint = root / "checkpoint.json"
        if not checkpoint.exists():
            return {"state": "unknown"}
        envelope = object_file(checkpoint)
        value = envelope.get("observation")
        if (set(envelope) != {"observation", "sha256"}
                or digest(canonical(value)) != envelope["sha256"]
                or value.get("schema") != schema_for(binding) or value.get("binding") != binding
                or value.get("kind") != "INCONCLUSIVE"):
            raise RuntimeError("checkpoint binding or digest differs")
        return {"state": "observed", "observation": value}
    raw = read(path)
    if read(sidecar, 256) != f"{digest(raw)}  observation.json\n".encode():
        raise RuntimeError("diagnostic sidecar differs")
    value = object_file(path)
    if value.get("schema") != schema_for(binding) or value.get("binding") != binding:
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
    if binding.get("product") == "AP1":
        from ap1_runtime import verify_runtime
    else:
        from pc0_diagnostic_runtime import verify_diagnostic_runner as verify_runtime
    runner = verify_runtime()
    if runner["baseline_contract_sha256"] != binding["runtime_identity"]:
        raise RuntimeError("runtime contract differs")
    if is_acceptance(binding) and runner["declared_inputs_sha256"] != binding["declared_runtime_inputs_sha256"]:
        raise RuntimeError("acceptance deployed runtime inputs differ")
    before = c.protected_snapshot()
    host_root = c.dx0_deck_host_artifact_parent() / binding["host_manifest_sha256"]
    build_input = a.read_canonical_json(host_root / "DX0_WINDOWS_HOST_BUILD_RECEIPT.json")["windows_build_input"]["sha256"]
    if binding.get("product") in {"AP0", "AP1"}:
        if binding.get("product") == "AP1":
            from ap1_worker_support import verify_host, bind_client
            bind_client(binding["native_client"])
        else:
            from ap0_worker_support import verify_host
    else:
        verify_host = a.verify_host_store
    host = verify_host(host_root, build_input)
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
    execution_sha = digest(canonical(diagnostic_input))
    if is_acceptance(binding) or binding.get("product") in {"AP0", "AP1"}:
        from pc0_contract import acceptance_execution_input_sha256
        execution_sha = acceptance_execution_input_sha256(binding,runner)
    return c, r, source, host, fixture, before, execution_sha, handoff, runner


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
        return {**retained, "effects": {"deck_workloads": 0, ("acceptance_publications" if is_acceptance(binding) else "diagnostic_publications"): 0}}
    c, r, source, host, fixture, before, deck_sha, _handoff, runtime = preflight(binding)
    import pc0_diagnostic_primitives as diagnostic
    ap0 = None
    if binding.get("product") in {"AP0", "AP1"}:
        if binding.get("product") == "AP1":
            import ap1_worker_support as ap0
        else:
            import ap0_worker_support as ap0
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
    from pc0_diagnostic_runtime import checkpoint_projection, exception_detail
    environment = observed = None
    retired = False
    contained = False
    available_saved = False
    retirement_disposition = "not_attempted_process_containment_unproved"
    cleanup, protected = "UNKNOWN", "UNKNOWN"
    after = None
    summary = None
    kind, classification = "INCONCLUSIVE", "PC0_DIAGNOSTIC_INCONCLUSIVE"
    trouble = {"schema": "pc0-read-only-troubleshooting/v1", "acceptance_eligible": False,
               "stage": "reserved", "last_event": None, "observation": None,
               "primary_error": None, "secondary_errors": [], "persistence_errors": [],
               "retirement": retirement_disposition}

    def error_at(stage, error):
        if error is None:
            return
        detail = {"stage": stage, "exception": exception_detail(error)}
        if trouble["primary_error"] is None:
            trouble["primary_error"] = detail
        elif len(trouble["secondary_errors"]) < 8:
            trouble["secondary_errors"].append(detail)

    def value_for(summary, kind="INCONCLUSIVE", classification="PC0_DIAGNOSTIC_INCONCLUSIVE"):
        return {"schema": schema_for(binding), "binding": binding, "execution_input_sha256": deck_sha,
                "kind": kind, "classification": classification, "summary": summary,
                "cleanup": cleanup, "protected": protected, "runtime_observation": runtime}

    def checkpoint(stage, available=None, error=None):
        nonlocal contained, available_saved
        trouble["stage"] = stage
        error_at(stage, error)
        if available is not None:
            available_saved = False
            trouble["observation"] = checkpoint_projection(available)
            records = available.get("records", [])
            if records:
                trouble["last_event"] = checkpoint_projection(records[-1])
            contained = available.get("cleanup") == {
                "owned_descendants_zero": True, "process_group_empty": True}
        value = value_for({"troubleshooting": trouble})
        raw = canonical({"observation": value, "sha256": digest(canonical(value))})
        temporary = None
        try:
            if len(raw) > (256 if binding.get("product") == "AP1" else 128) * 1024:
                raise RuntimeError("checkpoint exceeds declared byte bound")
            # One ordinary atomic file; failed writes leave the earlier checkpoint.
            with tempfile.NamedTemporaryFile(dir=root, prefix=".checkpoint-", delete=False) as stream:
                temporary = pathlib.Path(stream.name)
                stream.write(raw)
                stream.flush()
                os.fsync(stream.fileno())
            os.replace(temporary, root / "checkpoint.json")
            sync(root)
            available_saved = trouble["observation"] is not None
        except Exception as exc:
            if len(trouble["persistence_errors"]) < 4:
                trouble["persistence_errors"].append({"stage": stage, "exception": exception_detail(exc)})
        finally:
            if temporary is not None and temporary.exists():
                try:
                    temporary.unlink()
                except OSError:
                    pass

    checkpoint("process_guard")
    try:
        c.process_guard()
        trouble["stage"] = "create_environment"
        environment = diagnostic.create_dx0_environment(
            binding["reservation_identity"][:32], host=host, fixture=fixture,
            execution_source=binding["execution_source"], deck_execution_input_sha256=deck_sha,
            runner_identity_sha256=runtime["launch_critical_manifest_sha256"],
            **({"verify_host": ap0.verify_host} if ap0 else {}))
        trouble["stage"] = "supervise"
        observed = (getattr(ap0,"supervise",diagnostic.supervise) if ap0 else diagnostic.supervise)(environment, mode=ap0.MODE if ap0 else c.PC0_MODE, checkpoint=checkpoint,
            **({"profile":ap0} if ap0 else {}))
        checkpoint("observation_retained", observed)
        if observed.get("supervision_exception"):
            trouble["primary_error"] = trouble["primary_error"] or {
                "stage": "supervise", "exception": observed["supervision_exception"]}
        if observed["classification"] == "scanner_completed" and contained:
            trouble["stage"] = "normalize"
            if ap0:
                summary = ap0.normalize(observed)
            else:
                audio, _component, _timeline = diagnostic.normalize_wa0_positive(
                    observed, r._build_for_normalizer(source, host, fixture), pre_setup=True)
                summary = {"run_id": observed["run_id"], "processing_contract": audio["processing_contract"],
                           "shutdown": checkpoint_projection(observed["inherited_shutdown"]),
                           "raw_exit": observed["raw_exit"]}
                if is_acceptance(binding):
                    from pc0_contract import acceptance_facts
                    summary["verification"] = acceptance_facts(observed, audio, _component, _timeline["positive"])
            kind, classification = "SUCCESS", "PC0_DIAGNOSTIC_OBSERVED"
        else:
            kind, classification = "FAILED", "PC0_DIAGNOSTIC_FAILED"
    except Exception as exc:
        error_at(trouble["stage"], exc)
    finally:
        # Independent checkpoint precedes every retirement, including when a
        # supervisor's post-cleanup interpretation raised instead of returning.
        checkpoint("before_retirement")
        if environment is not None and contained and available_saved:
            try:
                retirement = diagnostic.retire_environment(environment,
                    runner_identity_sha256=runtime["launch_critical_manifest_sha256"])
                retired = retirement["environment_retired"] is True and retirement["stage_absent"] is True
                retirement_disposition = "retired" if retired else "failed"
            except Exception as exc:
                retirement_disposition = "failed"
                error_at("retirement", exc)
        elif environment is not None and contained:
            retirement_disposition = "not_attempted_diagnostic_not_retained"
        trouble["retirement"] = retirement_disposition
    try:
        after = c.protected_snapshot()
        protected = "UNCHANGED" if after == before else "CHANGED"
    except Exception as exc:
        error_at("protected_readback", exc)
    cleanup = "INCOMPLETE"
    if retired and contained:
        try:
            c.process_guard()
            cleanup = "COMPLETE"
        except Exception as exc:
            error_at("process_readback", exc)
    if ap0 and summary is None:
        summary = {"troubleshooting": trouble}
    if not ap0 and summary is None and observed is not None:
        try:
            summary = {"failure": failure_data(r, observed, binding, deck_sha, retirement_disposition),
                       "supervision_error": observed.get("supervision_error")}
        except Exception as exc:
            error_at("failure_report", exc)
    if (summary is None or trouble["primary_error"] is not None
            or trouble["persistence_errors"]):
        summary = {"troubleshooting": trouble}
        kind, classification = "INCONCLUSIVE", "PC0_DIAGNOSTIC_INCONCLUSIVE"
    if cleanup != "COMPLETE" or protected != "UNCHANGED":
        kind, classification = "INCONCLUSIVE", "PC0_DIAGNOSTIC_INCONCLUSIVE"
    if ap0 and kind == "SUCCESS":
        summary.update(stage_absent=retired, protected_before_sha256=digest(canonical(before)),
                       protected_after_sha256=digest(canonical(after)))
        try:
            ap0.validate_summary(summary)
        except Exception as exc:
            error_at("ap0_admission",exc)
            summary={"troubleshooting":trouble}
            kind="INCONCLUSIVE"
    if not ap0 and is_acceptance(binding) and kind == "SUCCESS":
        summary["verification"].update(stage_absent=retired,
            protected_before_sha256=digest(canonical(before)),
            protected_after_sha256=digest(canonical(after)))
        try:
            from pc0_contract import validate_acceptance_summary
            validate_acceptance_summary(summary)
        except Exception as exc:
            error_at("acceptance_admission", exc)
            summary = {"troubleshooting": trouble}
            kind = "INCONCLUSIVE"
    checkpoint("publication")
    if trouble["persistence_errors"]:
        summary = {"troubleshooting": trouble}
        kind, classification = "INCONCLUSIVE", "PC0_DIAGNOSTIC_INCONCLUSIVE"
    if is_acceptance(binding):
        classification = "PC0_ACCEPTANCE_OBSERVED" if kind == "SUCCESS" else "PC0_ACCEPTANCE_INCONCLUSIVE"
    if ap0:
        classification = binding["product"] + ("_SAMPLES_VERIFIED" if kind == "SUCCESS" else "_INCONCLUSIVE")
    value = value_for(summary, kind, classification)
    publications = 0
    try:
        raw = canonical(value)
        publish(root / "observation.json", raw)
        publish(root / "observation.sha256", f"{digest(raw)}  observation.json\n".encode())
        publications = 1
    except Exception as exc:
        error_at("publication", exc)
        checkpoint("publication_failed")
        value = value_for({"troubleshooting": trouble})
    return {"state": "observed", "observation": value,
            "effects": {"deck_workloads": 1 if observed is not None else "unknown",
                        ("acceptance_publications" if is_acceptance(binding) else "diagnostic_publications"): publications}}


def main(execution_class):
    action, encoded = sys.argv[2:4]
    binding = json.loads(bytes.fromhex(encoded))
    if binding["execution_class"] != execution_class:
        raise RuntimeError("worker execution class differs")
    if binding["worker_sha256"] != sys.argv[1]:
        raise RuntimeError("worker identity differs")
    proof = pathlib.Path.home() / ".local/share/linux-vst-bridge/proof"
    if action == "inspect":
        value = inspect(proof, binding)
    elif action == "preflight":
        runtime = preflight(binding)[-1]
        value = {"state": "ready", "runtime_observation": runtime}
    elif action == "execute":
        value = execute(proof, binding)
    else:
        raise RuntimeError("unsupported diagnostic action")
    raw = canonical(value)
    if len(raw) > MAX_BYTES:
        raise RuntimeError("diagnostic reply exceeds bound")
    sys.stdout.buffer.write(raw)


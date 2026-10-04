#!/usr/bin/env python3
"""Installed first-party configuration regression; raw outputs must stay private.

Run as the populated disposable fixture user. Input schema is in --example-config.
Only first-party reference modules and the independent SDK consumer belong here.
Product-state changes use offered managed actions. Export allow-listed summaries only.
No installer, service restart, source build, remote access, or automatic repair.
On failure, only this helper's SDK consumers are stopped; product state is retained.
"""
import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import re
import socket
import struct
import subprocess
import threading
import time
import traceback

SIBLING_BLOCKS = 2400
SETTINGS_SIBLING_BLOCKS = 4800
CONSUMER_BLOCKS = 480
HOST_FRAMES = 1024
SAMPLE_RATE = 48000
STARTUP_SECONDS = 60
RETIREMENT_SECONDS = 20
PRODUCT_READ_SECONDS = 45


class Failed(Exception):
    pass


def need(value, code):
    if not value:
        raise Failed(code)


def read(path):
    path = Path(path)
    need(path.stat().st_size <= 8 * 1024 * 1024, "json_extent")
    return json.loads(path.read_bytes())


def sha(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def save(path, value):
    Path(path).write_text(json.dumps(value, indent=2) + "\n")


def failure_details(error):
    result = {"type": type(error).__name__, "reason": str(error), "traceback": traceback.format_exc()}
    if isinstance(error, (subprocess.CalledProcessError, subprocess.TimeoutExpired)):
        result.update(command=error.cmd, returncode=getattr(error, "returncode", None), timeout=getattr(error, "timeout", None))
        for name, value in (("stdout", getattr(error, "output", None)), ("stderr", getattr(error, "stderr", None))):
            data = value.encode() if isinstance(value, str) else value or b""
            result[name] = {"text": data[:1048576].decode("utf-8", errors="replace"), "bytes": len(data),
                            "sha256": hashlib.sha256(data).hexdigest(), "truncated": len(data) > 1048576}
    return result


def exact_product_refresh(error, command):
    if (not isinstance(error, subprocess.CalledProcessError)
            or type(error.returncode) is not int or error.returncode != 1
            or error.cmd != command or error.output != b""
            or not isinstance(error.stderr, bytes) or len(error.stderr) > 1048576):
        return False
    refusal = b'Error: "operator_state_changed_refresh"'
    lines = error.stderr.splitlines()
    if lines.count(refusal) != 1:
        return False
    for line in lines:
        if line == refusal:
            continue
        if not line.startswith(b"PB0_PHASE "):
            return False
        try:
            phase = json.loads(line[len(b"PB0_PHASE "):])
        except (ValueError, UnicodeError):
            return False
        if type(phase) not in (dict, list):
            return False
    return True


def applied_launch(registration, report):
    policy = registration["environment"]["runner"].get("policy")
    need(policy in (None, "x11_touch_release_v1", "x11_touch_routing_v2"), "fixture_runner_has_other_graphics_policy")
    compatibility = registration["compatibility"]
    backend = compatibility.get("graphics")
    need(backend in (None, "wine_d3d11"), "fixture_graphics_choice")
    overrides = []
    if backend == "wine_d3d11":
        overrides.append("d3d11,dxgi=b")
    if compatibility["disable_windows_accessibility"]:
        overrides.append("uiautomationcore=")
    expected = {"requested_backend": backend, "dll_overrides": ";".join(overrides) or None,
                "scope": "host_process_and_children", "renderer_observed": False}
    need(report.get("graphics_configuration") == expected, "applied_launch_environment_mismatch")
    return expected


def offers(value):
    if isinstance(value, dict):
        if "action" in value and "disabled_reason" in value:
            yield value
        for child in value.values():
            yield from offers(child)
    elif isinstance(value, list):
        for child in value:
            yield from offers(child)


RELEASE_ARTIFACTS = {
    "manager": "usr/bin/linux-vst-bridge",
    "windows_host": "usr/lib/linux-vst-bridge/host/bridge-host.exe",
    "windows_host_source_manifest": "usr/lib/linux-vst-bridge/host/source-manifest.json",
    "native_engine": "usr/lib/linux-vst-bridge/proxy/ReusableEngine.so",
}


def release_binding(config):
    need(config["schema"] == 1 and re.fullmatch(r"[a-f0-9]{40}", config["source_head"]), "input_schema_or_source")
    need(re.fullmatch(r"[A-Za-z0-9.]+", config["package"]), "package_label")
    artifact = config["release_manifest"]
    need(re.fullmatch(r"[a-f0-9]{64}", artifact["sha256"])
         and sha(artifact["path"]) == artifact["sha256"], "release_manifest_digest_changed")
    release = read(artifact["path"])
    need(release["schema"] == 1 and release["source_head"] == config["source_head"]
         and release["version"] == config["package"]
         and re.fullmatch(r"[a-f0-9]{40}", release["source_tree"]), "release_source_or_package_mismatch")
    roster = {row["destination"]: row["sha256"] for row in release["files"]}
    need(len(roster) == len(release["files"]), "release_duplicate_destination")
    artifacts = {}
    for name, destination in RELEASE_ARTIFACTS.items():
        need(destination in roster and re.fullmatch(r"[a-f0-9]{64}", roster[destination]), "release_artifact_binding")
        artifacts[name] = roster[destination]
    return {"source_head": config["source_head"], "source_tree": release["source_tree"],
            "package": config["package"], "release_manifest": copy.deepcopy(artifact), "artifacts": artifacts}


def resume_configuration_binding(previous, current):
    # A different manager generation may resume exact retained preparation,
    # but cannot inherit another fixture, consumer, publication or runtime.
    permitted = {"source_head", "package", "release_manifest"}
    need(previous.keys() == current.keys()
         and {key: value for key, value in previous.items() if key not in permitted}
         == {key: value for key, value in current.items() if key not in permitted}, "resume_input_changed")
    before, after = release_binding(previous), release_binding(current)
    unchanged = ("native_engine", "windows_host", "windows_host_source_manifest")
    need(all(before["artifacts"][name] == after["artifacts"][name] for name in unchanged),
         "resume_execution_artifacts_changed")
    need(all(fixture["native_sha256"] == before["artifacts"]["native_engine"]
             for fixture in (previous["target"], previous["sibling"])), "resume_fixture_engine_binding")
    changed = sorted(key for key in permitted if previous[key] != current[key])
    return {"manager_update": bool(changed), "changed_input_fields": changed,
            "previous": before, "current": after,
            "unchanged_execution_artifacts": {name: before["artifacts"][name] for name in unchanged}}


EXAMPLE = {
    "schema": 1, "source_head": "FULL_FROZEN_SOURCE_COMMIT", "package": "EXACT_PACKAGE_VERSION",
    "manager": "/usr/bin/linux-vst-bridge",
    "root": "/home/FIXTURE/.local/share/linux-vst-bridge/managed",
    "release_manifest": {"path": "/PRIVATE/RELEASE_MANIFEST.json", "sha256": "SHA256"},
    "host": {"path": "/PRIVATE/lifecycle-host", "sha256": "SHA256"},
    "audit": {"path": "/PRIVATE/libap3-callback-audit.so", "sha256": "SHA256"},
    "target": {"environment": "EXACT_ENVIRONMENT", "class_id": "WINDOWS_CLASS_32_HEX",
               "module_sha256": "SHA256", "native_sha256": "SHA256", "role": "instrument",
               "bundle": "/home/FIXTURE/.vst3/LVB_CLASS.vst3",
               "processor_id": "NATIVE_PROCESSOR_32_HEX", "controller_id": "NATIVE_CONTROLLER_32_HEX",
               "publication": {"id": "EXACT_BASELINE_ID", "sha256": "SHA256"},
               "settings": {"graphics": None, "accessibility": "profile_default"}},
    "sibling": {"environment": "SAME_EXACT_ENVIRONMENT", "class_id": "OTHER_WINDOWS_CLASS_32_HEX",
                "module_sha256": "SHA256", "native_sha256": "SHA256", "role": "effect",
                "bundle": "/home/FIXTURE/.vst3/LVB_OTHER_CLASS.vst3",
                "processor_id": "NATIVE_PROCESSOR_32_HEX", "controller_id": "NATIVE_CONTROLLER_32_HEX",
                "publication": {"id": "EXACT_SIBLING_ID", "sha256": "SHA256"},
                "settings": {"graphics": None, "accessibility": "profile_default"}}
}


class Run:
    def __init__(self, config, output, single, resume=None):
        self.c, self.out, self.single = config, Path(output), single
        self.out.mkdir(mode=0o700, parents=True, exist_ok=False)
        self.root = Path(config["root"])
        self.target, self.sibling = config["target"], config["sibling"]
        self.children, self.counter, self.audio_runs = [], 0, []
        self.trials = []
        self.resume = Path(resume).resolve(strict=True) if resume else None
        self.reused_audio = []
        self.resume_binding = None
        save(self.out / "input.json", config)
        self.env = os.environ.copy()
        desktop = self.command("desktop-environment", ["systemctl", "--user", "show-environment"], timeout=5, retain_stdout=False).decode()
        for line in desktop.splitlines():
            key, _, value = line.partition("=")
            if key in ("DISPLAY", "XAUTHORITY", "WAYLAND_DISPLAY", "DBUS_SESSION_BUS_ADDRESS"):
                self.env[key] = value
        self.env["LD_PRELOAD"] = config["audit"]["path"]

    def event(self, event, **facts):
        row = {"event": event, "monotonic_ns": time.monotonic_ns(), **facts}
        with (self.out / "summary.ndjson").open("a") as stream:
            stream.write(json.dumps(row) + "\n")
        print(json.dumps(row), flush=True)

    def command(self, label, argv, input=None, timeout=45, retain_stdout=True):
        began = time.monotonic_ns()
        try:
            result = subprocess.run(argv, input=input, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=timeout, check=True)
        except (subprocess.CalledProcessError, subprocess.TimeoutExpired) as error:
            details = failure_details(error)
            if not retain_stdout:
                details["stdout"].pop("text", None)
                save(self.out / (label + "-command-failure.json"), details)
                raise Failed("desktop_environment_unavailable") from None
            save(self.out / (label + "-command-failure.json"), details)
            raise
        finally:
            save(self.out / (label + "-command-interval.json"), {"began_ns": began, "ended_ns": time.monotonic_ns()})
        for name, data, limit in (("stdout", result.stdout, 8 * 1024 * 1024), ("stderr", result.stderr, 1048576)):
            if name != "stdout" or retain_stdout:
                (self.out / (label + "-" + name + ".log")).write_bytes(data[:limit])
            need(len(data) <= limit, "command_output_extent")
        return result.stdout

    def artifact(self, artifact):
        need(re.fullmatch(r"[0-9a-f]{64}", artifact["sha256"]), "artifact_digest_syntax")
        need(sha(artifact["path"]) == artifact["sha256"], "artifact_digest_changed")

    def entry(self, fixture):
        return read(self.root / "registry.json")["classes"][fixture["class_id"]]

    def performance(self, fixture):
        path = self.root / "performance" / (fixture["class_id"] + ".json")
        return read(path) if path.exists() else {"schema": 1, "added_frames": 512}

    def capacity(self, expected=None):
        def receive(peer, count):
            data = b""
            while len(data) < count:
                chunk = peer.recv(count - len(data))
                need(chunk, "capacity_response_incomplete")
                data += chunk
            return data
        with socket.socket(socket.AF_UNIX) as peer:
            peer.settimeout(5)
            peer.connect(str(self.root / "runtime/owner.sock"))
            peer.sendall(b"LVC1\n")
            size, = struct.unpack("<I", receive(peer, 4))
            need(size <= 65536, "capacity_extent")
            raw = json.loads(receive(peer, size))
        need(raw.get("ok") is True, "capacity_unavailable")
        value = raw["capacity"]
        need(value["maintenance"] == 0 and not value["cleanup_unconfirmed"], "maintenance_or_unresolved_cleanup")
        if expected is not None:
            need(value["dsp"] == expected, "unexpected_dsp_count")
        need(not (self.root / "operator/resume.json").exists(), "pending_operator_recovery")
        return value

    def leases(self):
        paths = list((self.root / "runtime/leases").glob("*.json"))
        need(len(paths) <= 64, "lease_extent")
        result = set()
        for path in paths:
            sid = path.stem
            need(re.fullmatch(r"[a-f0-9]{32}", sid), "lease_identity")
            try:
                report = Path(read(path))
            except FileNotFoundError:
                continue
            need(report.parent == self.root / "runtime/results", "lease_report_location")
            if report.name == "windows-" + sid + ".json":
                result.add(sid)
            else:
                need(report.name == "environment-" + sid + ".json", "lease_report_kind")
        return result

    def detail(self, fixture, label):
        command = [self.c["manager"], "operator", "product", fixture["environment"],
                   fixture["module_sha256"], fixture["class_id"]]
        deadline_ns = time.monotonic_ns() + PRODUCT_READ_SECONDS * 1_000_000_000
        attempts = (label + "-product", label + "-product-read-2")
        for attempt, stem in enumerate(attempts):
            remaining = (deadline_ns - time.monotonic_ns()) / 1_000_000_000
            if remaining <= 0:
                error = subprocess.TimeoutExpired(command, PRODUCT_READ_SECONDS)
                failure = failure_details(error)
                failure.update(command_started=False, deadline_ns=deadline_ns)
                save(self.out / (stem + "-command-failure.json"), failure)
                raise error
            try:
                data = self.command(stem, command, timeout=remaining)
            except subprocess.CalledProcessError as error:
                if attempt or not exact_product_refresh(error, command):
                    raise
                save(self.out / (label + "-product-refresh.json"),
                     {"schema": 1, "refusal": "operator_state_changed_refresh",
                      "first_attempt": attempts[0], "second_attempt": attempts[1],
                      "deadline_ns": deadline_ns})
                # Repeat only this read, within its original deadline. A second
                # refusal or any request/audio failure remains terminal here.
                continue
            break
        need(len(data) <= 8 * 1024 * 1024, "product_extent")
        value = json.loads(data)
        if attempt:
            save(self.out / (stem + ".json"), value)
        save(self.out / (label + "-product.json"), value)
        need(value["schema"] == 1 and value["operator_schema"] in (18, 19), "operator_schema")
        return value

    def select(self, detail, predicate, disabled=False):
        matches = {json.dumps(row["action"], sort_keys=True): row for row in offers(detail)
                   if predicate(row["action"])}
        # Both existing publication controls can offer the same replacement.
        # Prefer the ordinary test-publication flow; never rewrite either action.
        if {row["action"]["kind"] for row in matches.values()} == {"experimental_replace", "compatibility_publish_test"}:
            matches = {key: row for key, row in matches.items() if row["action"]["kind"] == "compatibility_publish_test"}
        need(len(matches) == 1, "exact_offer_absent_or_ambiguous")
        offer = next(iter(matches.values()))
        need(bool(offer["disabled_reason"]) == disabled, "offer_availability_changed")
        return offer

    def action(self, label, predicate, mutate=None, disabled=False, active=None):
        self.counter += 1
        stem = f"{self.counter:02d}-{label}"
        if disabled:
            before = {"target": self.entry(self.target), "sibling": self.entry(self.sibling),
                      "target_buffering": self.performance(self.target), "sibling_buffering": self.performance(self.sibling)}
        detail = self.detail(self.target, stem)
        offer = self.select(detail, predicate, disabled)
        action = copy.deepcopy(offer["action"])
        if mutate:
            mutate(action)
        request = {"schema": detail["operator_schema"], "state_token": detail["state_token"], "action": action}
        save(self.out / (stem + "-request.json"), request)
        if active:
            active.assert_processing()
        began = time.monotonic_ns()
        raw = self.command(stem + "-request", [self.c["manager"], "operator", "request"], input=json.dumps(request).encode())
        receipt = json.loads(raw)
        receipt_observed = time.monotonic_ns()
        save(self.out / (stem + "-receipt.json"), receipt)
        need(receipt["schema"] == request["schema"], "receipt_schema")
        operation = receipt["operation"]
        need(isinstance(operation, str) and re.fullmatch(r"[a-f0-9]{32}", operation), "operator_operation_identity")
        directory = self.root / "operator" / operation
        recorded_request = read(directory / "request.json")
        save(self.out / (stem + "-recorded-request.json"), recorded_request)
        need(recorded_request == request, "operator_recorded_request_changed")
        if disabled:
            need(offer["disabled_reason"] == "Close instances of this plug-in before changing its configuration", "not_target_busy_offer")
            need(receipt["accepted"] is False
                 and receipt["refusal"] == offer["disabled_reason"], "not_exact_target_busy_refusal")
            result = read(directory / "result.json")
            save(self.out / (stem + "-result.json"), result)
            need(result["schema"] == 1 and result["operation"] == operation and result["state"] == "refused"
                 and result["stage"] == "operator_request_admission" and result["action"] == action
                 and result["reason"] == offer["disabled_reason"]
                 and result["worker_started"] is False and result["mutation_started"] is False,
                 "not_exact_durable_target_busy_refusal")
            need(before == {"target": self.entry(self.target), "sibling": self.entry(self.sibling),
                            "target_buffering": self.performance(self.target), "sibling_buffering": self.performance(self.sibling)},
                 "target_busy_refusal_changed_selection_or_buffering")
        else:
            need(receipt["accepted"] is True and receipt["refusal"] is None, "operator_request_refused")
            until = time.monotonic() + 120
            result = None
            while time.monotonic() < until:
                path = directory / "result.json"
                try:
                    result = read(path)
                except FileNotFoundError:
                    pass
                if result and result.get("state") in ("completed", "refused", "failed", "interrupted"):
                    break
                if active:
                    active.assert_processing()
                time.sleep(0.1)
            save(self.out / (stem + "-result.json"), result)
            need(result and result.get("schema") == 1 and result.get("operation") == operation
                 and result.get("state") == "completed", "operator_operation_not_completed")
        ended = time.monotonic_ns()
        save(self.out / (stem + "-interval.json"), {"began_ns": began, "receipt_observed_ns": receipt_observed, "ended_ns": ended})
        if active:
            active.assert_processing()
            active.intervals.append((label, began, ended))
        self.event("operator", step=label, kind=action["kind"], accepted=receipt["accepted"], state=result["state"])
        return result.get("result", {}), action

    def published_artifacts(self, fixture, registration):
        native = Path(fixture["bundle"]) / "Contents/x86_64-linux" / ("LVB_" + fixture["class_id"] + ".so")
        self.artifact(registration["native"])
        need(sha(native) == fixture["native_sha256"] == registration["native"]["sha256"], "published_native_digest")
        need(native.resolve(strict=True) == Path(registration["native"]["path"]).resolve(strict=True), "published_native_location")
        if "descriptor" in registration:
            artifact = registration["descriptor"]
            self.artifact(artifact)
            descriptor_path = Path(artifact["path"])
            need(descriptor_path.parent == Path(registration["native"]["path"]).parent, "published_descriptor_location")
            published = native.parent / descriptor_path.name
            need(published.resolve(strict=True) == descriptor_path.resolve(strict=True), "published_descriptor_selection")
            descriptor = read(descriptor_path)
            need(descriptor["class_id"] == fixture["class_id"]
                 and descriptor["module_sha256"] == fixture["module_sha256"]
                 and descriptor["engine_sha256"] == registration["native"]["sha256"], "published_descriptor_identity")

    def check_fixture(self, fixture, label):
        need(fixture["role"] in ("effect", "instrument"), "fixture_role")
        for key in ("class_id", "processor_id", "controller_id"):
            need(re.fullmatch(r"[0-9A-F]{32}", fixture[key]), "fixture_class_identity")
        entry = self.entry(fixture)
        need(entry["publication"] == "Published" and entry["managed_revision"] == fixture["publication"], "fixture_publication")
        reg = entry["registration"]
        need(reg["environment"]["id"] == fixture["environment"] and reg["module"]["sha256"] == fixture["module_sha256"]
             and reg["native"]["sha256"] == fixture["native_sha256"], "fixture_identity")
        need(reg["compatibility"].get("graphics") is None
             and reg["compatibility"]["disable_windows_accessibility"] is False,
             "baseline_launch_defaults_required")
        reference = fixture["publication"]
        need(re.fullmatch(r"[a-f0-9]{32}", reference["id"])
             and re.fullmatch(r"[a-f0-9]{64}", reference["sha256"]), "baseline_publication_identity")
        revision_path = self.root / "publications" / fixture["class_id"] / "revisions" / reference["id"] / "revision.json"
        self.artifact({"path": str(revision_path), "sha256": reference["sha256"]})
        revision = read(revision_path)
        external_ids = revision["external_ids"]
        need(isinstance(external_ids, list) and len(external_ids) == 2
             and all(isinstance(value, str) and re.fullmatch(r"[0-9A-Fa-f]{32}", value)
                     and value.upper() == expected
                     for value, expected in zip(external_ids, (fixture["processor_id"], fixture["controller_id"]))),
             "baseline_retained_external_ids_changed")
        need(revision["id"] == reference["id"] and revision["class_id"] == fixture["class_id"]
             and revision["registration"] == reg, "baseline_retained_entry_changed")
        need(Path(fixture["bundle"]).is_symlink()
             and Path(fixture["bundle"]).resolve(strict=True) == Path(revision["target"]).resolve(strict=True), "baseline_publication_target_changed")
        for name in ("module", "host", "native", "descriptor"):
            if name in reg:
                self.artifact(reg[name])
        # Preparation-kit and selected-software hosts are independently verified
        # copies. Product execution pairing uses component bytes plus source identity.
        need(reg["host"]["sha256"] == self.software["host"]["sha256"]
             and reg["host_source_sha256"] == self.software["source_sha256"], "fixture_host_not_selected_package")
        self.published_artifacts(fixture, reg)
        detail = self.detail(fixture, label)
        product = detail["product"]
        selected = product["details"]
        need(product["class_id"] == fixture["class_id"] and product["environment"] == fixture["environment"]
             and product["module_sha256"] == fixture["module_sha256"]
             and selected["publication"] == reference
             and selected["publication_selected"] is True and selected["publication_valid"] is True
             and selected.get("refusal") is None, "baseline_selected_publication_unavailable")
        configuration = selected["configuration"]
        need(configuration["publication"] in ("ordinary", "experimental", "another_configuration"), "baseline_configuration_unavailable")
        if configuration["publication"] != "another_configuration":
            need(configuration["settings"] == fixture["settings"], "baseline_settings")
        # Configuration shows the newest workflow candidate, which may be an
        # unselected retained trial. The exact selected revision above owns the
        # baseline and its effective launch defaults independently of that view.
        save(self.out / (label + "-selected-baseline.json"), {"publication": reference,
             "registration": reg, "settings": fixture["settings"], "effective_launch_defaults": reg["compatibility"]})
        need(self.performance(fixture) == {"schema": 1, "added_frames": 1024}, "initial_1024_buffering_required")
        return entry

    def prior_consumer(self, directory, registration):
        path = directory / "events.json"
        if not path.exists():
            path = directory / "partial-events.json"
        rows = read(path)
        need(rows and rows[-1]["event"] == "passed", "resume_prior_consumer_not_passed")
        audio, = [row for row in rows if row["event"] == "audio"]
        need(audio["callback_audited"] and audio["mismatches"] == audio["nonfinite"] == audio["rejected_callbacks"] == 0
             and audio["nonzero"] > 0, "resume_prior_audio_failed")
        path = directory / "retirement.json"
        report = read(path)["report"] if path.exists() else read(directory / "partial-retirement.json")
        need(report["cleanup_confirmed"] is True and report["transport_retired"] is True, "resume_prior_retirement_missing")
        owner = read(directory / "owner.json")
        expected_binding = {key: registration[key] for key in
                            ("environment", "module", "host", "host_source_sha256", "compatibility")}
        expected_binding["metadata"] = {"class_id": registration["metadata"]["class_id"]}
        need(owner["keeper"] is False and owner["inspect"] is False
             and owner["registration"] == expected_binding, "resume_prior_owner_binding")
        session = report["session"]
        need(owner["session"] == session
             and owner["report"] == str(self.root / "runtime/results" / ("windows-" + session + ".json")), "resume_prior_report_binding")
        applied_launch(expected_binding, report)
        need(re.fullmatch(r"[a-f0-9]{32}", session) and session not in self.leases(), "resume_prior_session_not_retired")
        phases_path = self.root / "runtime/results" / ("native-" + session + ".jsonl")
        need(phases_path.stat().st_size <= 131072, "resume_prior_phases_extent")
        phases = []
        for line in phases_path.read_bytes().splitlines():
            try:
                row = json.loads(line)
            except (ValueError, UnicodeError):
                continue
            if row.get("event") == "ap7_audio_phase":
                phases.append(row)
        need(phases and phases[-1]["phase"] == "retired"
             and all(row.get("processing", {}).get("missing_frames", 0) == 0 for row in phases), "resume_prior_native_retirement")
        self.reused_audio.append(audio["compared_samples"])
        return rows

    def resume_prepared(self, baseline, sibling_entry, settings):
        prior = self.resume
        previous_input = (prior / "input.json").read_bytes()
        need(len(previous_input) <= 8 * 1024 * 1024, "resume_input_extent")
        self.resume_binding = resume_configuration_binding(json.loads(previous_input), self.c)
        (self.out / "resume-prior-input.json").write_bytes(previous_input)
        save(self.out / "resume-artifact-identities.json", self.resume_binding)
        need(read(prior / "baseline-entries.json") == {"target": baseline, "sibling": sibling_entry}, "resume_baseline_changed")
        for name in ("baseline-record", "baseline-recall"):
            self.prior_consumer(prior / name, baseline["registration"])
        results = list(prior.glob("*-trial-1-prepare-result.json"))
        need(len(results) == 1, "resume_preparation_result_absent_or_ambiguous")
        result_path = results[0]
        result = read(result_path)
        request = read(result_path.with_name(result_path.name.replace("-result.json", "-request.json")))
        receipt = read(result_path.with_name(result_path.name.replace("-result.json", "-receipt.json")))
        action = request["action"]
        need(receipt["accepted"] is True and result["state"] == "completed"
             and result["operation"] == receipt["operation"], "resume_preparation_not_completed")
        candidate = result["result"]["candidate"]
        need(re.fullmatch(r"[a-f0-9]{64}", candidate) and action["kind"] == "candidate_settings_prepare"
             and action["settings"] == settings and action["expected_current"] == baseline["managed_revision"], "resume_preparation_binding")
        detail = self.detail(self.target, "resume-prepared")
        configuration = detail["product"]["details"]["configuration"]
        need(configuration["candidate"] == candidate and configuration["settings"] == settings
             and configuration["publication"] == "another_configuration"
             and configuration["change"] == {"predecessor": action["candidate"], "baseline": baseline["managed_revision"]}, "resume_retained_trial_binding")
        self.select(detail, lambda a: a["kind"] in ("compatibility_publish_test", "experimental_replace")
                    and a["candidate"] == candidate and a["expected_current"] == baseline["managed_revision"])
        sibling_directory = prior / "trial-1-sibling"
        if not sibling_directory.exists():
            sibling_directory = prior / "trial-1-prepare-sibling"
        rows = self.prior_consumer(sibling_directory, sibling_entry["registration"])
        timing, = [row for row in rows if row["event"] == "consumer_timing"]
        summary_path = prior / "summary.ndjson"
        need(summary_path.stat().st_size <= 1048576, "resume_summary_extent")
        summary = [json.loads(line) for line in summary_path.read_bytes().splitlines()]
        started, = [row["monotonic_ns"] for row in summary if row["event"] == "processing" and row["run"] == sibling_directory.name]
        ended, = [row["monotonic_ns"] for row in summary if row["event"] == "operator" and row["step"] == "trial-1-prepare" and row["state"] == "completed"]
        audio_start = timing["audio_begin_monotonic_ns"]
        audio_end = audio_start + timing["blocks"][-1]["started_ns"]
        need(audio_start < started < ended < audio_end, "resume_prepare_outside_sibling_audio")
        prefix = prior / "original-state"
        self.original = {part: sha(prefix.with_suffix("." + part)) for part in ("component", "controller")}
        checkpoint = prior / "baseline-checkpoint.json"
        if checkpoint.exists():
            need(read(checkpoint)["original_state_sha256"] == self.original, "resume_saved_state_changed")
        save(self.out / "resume.json", {"source": str(prior), "candidate": candidate, "preparation_operation": receipt["operation"],
             "original_prefix": str(prefix), "original_state_sha256": self.original,
             "earlier_hash_checkpoint": checkpoint.exists(), "inherited_prepare_interval": [started, ended],
             "previous_input_sha256": hashlib.sha256(previous_input).hexdigest(),
             "artifact_identities": self.resume_binding,
             "inherited_results": {"source_head": self.resume_binding["previous"]["source_head"],
                                   "package": self.resume_binding["previous"]["package"],
                                   "audio_runs": len(self.reused_audio), "compared_samples": sum(self.reused_audio)}})
        # Old helpers did not persist the in-memory state hashes on failure.
        # Confirm recall from the exact retained files and never rewrite them.
        Consumer(self, self.target, "resume-baseline-recall", "recall", prefix).finish()
        self.check_original(prefix)
        self.event("resumed_preparation", candidate=candidate, baseline_recall_confirmed=True,
                   source_head=self.c["source_head"], package=self.c["package"],
                   inherited_source_head=self.resume_binding["previous"]["source_head"],
                   inherited_package=self.resume_binding["previous"]["package"],
                   inherited_audio_runs=len(self.reused_audio), inherited_compared_samples=sum(self.reused_audio))
        return prefix, candidate

    def run(self):
        need(self.c["schema"] == 1 and re.fullmatch(r"[a-f0-9]{40}", self.c["source_head"]), "input_schema_or_source")
        need(re.fullmatch(r"[A-Za-z0-9.]+", self.c["package"]), "package_label")
        for key in ("host", "audit", "release_manifest"):
            self.artifact(self.c[key])
        roster = release_binding(self.c)["artifacts"]
        self.software = read(self.root / "software.json")
        for name, destination in (("manager", "manager"), ("host", "windows_host")):
            self.artifact(self.software[name])
            need(self.software[name]["sha256"] == roster[destination], "selected_package_mismatch")
        self.artifact(self.software["source_manifest"])
        need(self.software["source_sha256"] == self.software["source_manifest"]["sha256"]
             == roster["windows_host_source_manifest"], "selected_host_source_pairing")
        need(sha(self.c["manager"]) == self.software["manager"]["sha256"], "manager_executable_mismatch")
        need(self.target["class_id"] != self.sibling["class_id"]
             and self.target["environment"] == self.sibling["environment"], "distinct_shared_environment_classes_required")
        need(all(fixture["native_sha256"] == roster["native_engine"]
                 for fixture in (self.target, self.sibling)), "fixture_engine_not_selected_package")
        self.capacity(0)
        baseline = self.check_fixture(self.target, "baseline-target")
        sibling_entry = self.check_fixture(self.sibling, "baseline-sibling")
        save(self.out / "baseline-entries.json", {"target": baseline, "sibling": sibling_entry})
        settings = self.target["settings"]
        need(settings["graphics"] is None and settings["accessibility"] != "disabled_for_host", "two_trial_baseline_required")
        desired = [{**settings, "graphics": "wine_d3d11"}, {"graphics": "wine_d3d11", "accessibility": "disabled_for_host"}]
        if self.single:
            desired = desired[:1]
        resumed_candidate = None
        if self.resume:
            prefix, resumed_candidate = self.resume_prepared(baseline, sibling_entry, desired[0])
        else:
            first_detail = self.detail(self.target, "preflight")
            self.select(first_detail, lambda a: a["kind"] == "candidate_settings_prepare" and a["settings"] == desired[0])
            prefix = self.out / "original-state"
            Consumer(self, self.target, "baseline-record", "record", prefix).finish()
            self.original = {part: sha(prefix.with_suffix("." + part)) for part in ("component", "controller")}
            Consumer(self, self.target, "baseline-recall", "recall", prefix).finish()
        save(self.out / "baseline-checkpoint.json", {"original_prefix": str(prefix), "original_state_sha256": self.original})
        for index, settings in enumerate(desired, 1):
            before = self.entry(self.target)
            if index == 1 and resumed_candidate:
                candidate = resumed_candidate
            else:
                preparing = Consumer(self, self.sibling, f"trial-{index}-prepare-sibling", "sibling", self.out / f"prepare-sibling-state-{index}",
                                     sibling_blocks=SETTINGS_SIBLING_BLOCKS)
                prepared, _ = self.action(f"trial-{index}-prepare", lambda a: a["kind"] == "candidate_settings_prepare"
                                          and a["settings"] == settings and a["expected_current"] == before["managed_revision"], active=preparing)
                candidate = prepared["candidate"]
                preparing.finish()
            need(re.fullmatch(r"[a-f0-9]{64}", candidate), "prepared_candidate_identity")
            need(self.entry(self.target) == before and self.entry(self.sibling) == sibling_entry, "prepare_changed_publication")
            need(self.performance(self.target)["added_frames"] == 1024, "prepare_changed_buffering")
            sibling = Consumer(self, self.sibling, f"trial-{index}-apply-sibling", "sibling", self.out / f"apply-sibling-state-{index}",
                               sibling_blocks=SETTINGS_SIBLING_BLOCKS)
            published, _ = self.action(f"trial-{index}-apply", lambda a: a["kind"] in ("experimental_replace", "compatibility_publish_test")
                                       and a["candidate"] == candidate and a["expected_current"] == before["managed_revision"], active=sibling)
            current = self.entry(self.target)
            need(current["managed_revision"] == published["publication"]
                 and current["managed_revision"] != before["managed_revision"], "trial_publication_readback")
            reg, old = current["registration"], before["registration"]
            for key in ("environment", "module", "host", "host_source_sha256", "metadata"):
                need(reg.get(key) == old.get(key), "trial_changed_exact_execution_identity")
            # Publication owns a new immutable bundle path for unchanged native
            # and descriptor bytes; restoration below still requires exact paths.
            for key in ("native", "descriptor"):
                need((key in reg) == (key in old), "trial_changed_artifact_kind")
                if key in reg:
                    need(reg[key]["sha256"] == old[key]["sha256"], "trial_changed_native_or_descriptor")
            self.published_artifacts(self.target, reg)
            save(self.out / f"trial-{index}-published-entry.json", current)
            expected_compatibility = {**old["compatibility"], "graphics": "wine_d3d11"}
            if settings["accessibility"] == "disabled_for_host":
                expected_compatibility["disable_windows_accessibility"] = True
            need(reg["compatibility"] == expected_compatibility, "trial_launch_choices")
            need(self.entry(self.sibling) == sibling_entry, "trial_changed_sibling")
            self.trials.append((candidate, before))
            sibling.finish()
            active = Consumer(self, self.target, f"trial-{index}-restore-active", "sibling", self.out / f"restore-active-state-{index}")
            self.action(f"trial-{index}-restore-refused", lambda a: a["kind"] == "experimental_disable" and a["candidate"] == candidate,
                        disabled=True, active=active)
            active.finish()
            need(self.entry(self.target) == current and self.performance(self.target)["added_frames"] == 1024, "restore_refusal_mutated_selection")
            active = Consumer(self, self.target, f"trial-{index}-buffering-active", "sibling", self.out / f"buffering-active-state-{index}")
            self.action(f"trial-{index}-buffering-refused", lambda a: a["kind"] == "buffering_set"
                        and a["class_id"] == self.target["class_id"] and a["added_frames"] == 512, disabled=True, active=active)
            active.finish()
            need(self.entry(self.target) == current and self.performance(self.target)["added_frames"] == 1024, "buffering_refusal_mutated_selection")
            Consumer(self, self.target, f"trial-{index}-original-recall", "recall", prefix).finish()
            self.check_original(prefix)
            def result_fields(action):
                action.update(result={"kind": "worked"}, passed=["audio", "parameters", "automation", "state_recall", "retirement"]
                              + (["midi"] if self.target["role"] == "instrument" else []), failed_area=None,
                              note="Installed SDK reference consumer verified audio, controls, original-state recall and owned retirement. No DAW project or editor claim.")
            kept, _ = self.action(f"trial-{index}-keep", lambda a: a["kind"] == "compatibility_result"
                                  and a["candidate"] == candidate and a["expected_current"] == current["managed_revision"], mutate=result_fields)
            need(kept["result"] == "partial_experimental" and kept["publication_changed"] is False
                 and kept["ordinary_published"] is False and self.entry(self.target) == current, "keep_qualification_or_publication_changed")
        self.set_buffering(512)
        for index, (candidate, before) in enumerate(reversed(self.trials), 1):
            self.action(f"restore-{index}", lambda a: a["kind"] == "experimental_disable" and a["candidate"] == candidate)
            need(self.entry(self.target) == before, "restore_not_exact_predecessor")
            need(self.performance(self.target) == {"schema": 1, "added_frames": 512}, "restore_lost_buffering_preference")
            need(self.entry(self.sibling) == sibling_entry, "restore_changed_sibling")
        need(self.entry(self.target) == baseline, "baseline_not_restored")
        self.set_buffering(1024)
        Consumer(self, self.target, "restored-original-recall", "recall", prefix).finish()
        self.check_original(prefix)
        self.capacity(0)
        self.detail(self.target, "final")
        buffering = self.performance(self.target)
        sibling_buffering = self.performance(self.sibling)
        need(buffering == {"schema": 1, "added_frames": 1024}, "final_target_buffering_required")
        need(sibling_buffering == {"schema": 1, "added_frames": 1024}, "final_sibling_buffering_required")
        save(self.out / "final-state.json", {"target": self.entry(self.target), "sibling": self.entry(self.sibling),
                                           "buffering": buffering, "sibling_buffering": sibling_buffering,
                                           "original_state_sha256": self.original})
        self.event("passed", source_head=self.c["source_head"], package=self.c["package"], trials=len(self.trials),
                   target_class=self.target["class_id"], sibling_class=self.sibling["class_id"],
                   subset=self.single, audio_runs=len(self.audio_runs), compared_samples=sum(self.audio_runs), mismatches=0,
                   inherited_source_head=self.resume_binding["previous"]["source_head"] if self.resume_binding else None,
                   inherited_package=self.resume_binding["previous"]["package"] if self.resume_binding else None,
                   inherited_audio_runs=len(self.reused_audio), inherited_compared_samples=sum(self.reused_audio),
                   baseline_restored=True, buffering_retained=True, original_state_unchanged=True,
                   claim="installed_configuration_sdk_regression", real_daw_project_claim=False, editor_claim=False)

    def check_original(self, prefix):
        need(all(sha(prefix.with_suffix("." + part)) == digest for part, digest in self.original.items()), "original_saved_state_changed")

    def set_buffering(self, frames):
        self.action("buffering-" + str(frames), lambda a: a["kind"] == "buffering_set"
                    and a["class_id"] == self.target["class_id"] and a["added_frames"] == frames)
        need(self.performance(self.target) == {"schema": 1, "added_frames": frames}, "buffering_readback")


class Consumer:
    def __init__(self, run, fixture, label, mode, prefix, sibling_blocks=SIBLING_BLOCKS):
        self.r, self.fixture, self.label, self.mode = run, fixture, label, mode
        self.events, self.intervals, self.errors = [], [], []
        self.finished = False
        need(type(sibling_blocks) is int and 2400 <= sibling_blocks <= 4800
             and (mode == "sibling" or sibling_blocks == SIBLING_BLOCKS), "consumer_declared_count")
        self.blocks = sibling_blocks if mode == "sibling" else CONSUMER_BLOCKS
        run.capacity(0)
        self.before = run.leases()
        self.registration = run.entry(fixture)["registration"]
        self.dir = run.out / label
        self.dir.mkdir(mode=0o700)
        command = [run.c["host"]["path"], fixture["bundle"], fixture["role"], mode, str(prefix), str(HOST_FRAMES),
                   fixture["processor_id"], fixture["controller_id"]]
        if mode == "sibling":
            command.append(str(self.blocks))
        save(self.dir / "command.json", command)
        self.started = time.monotonic()
        self.deadline = self.started + STARTUP_SECONDS + self.blocks * HOST_FRAMES / SAMPLE_RATE + RETIREMENT_SECONDS
        self.child = subprocess.Popen(command, env=run.env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        run.children.append(self)
        self.threads = [threading.Thread(target=self.drain, args=(self.child.stdout, "stdout", True), daemon=True),
                        threading.Thread(target=self.drain, args=(self.child.stderr, "stderr", False), daemon=True)]
        for thread in self.threads:
            thread.start()
        until = self.started + STARTUP_SECONDS
        while time.monotonic() < until:
            if any(row.get("event") == "processing_state" for _, row in self.events):
                break
            need(self.child.poll() is None and not self.errors, "consumer_failed_before_processing")
            time.sleep(0.05)
        else:
            raise Failed("consumer_processing_deadline")
        sessions = run.leases() - self.before
        need(len(sessions) == 1, "consumer_exact_live_session")
        self.session = next(iter(sessions))
        owner_path = Path(self.registration["environment"]["root"]) / "compatdata/pfx/drive_c/bridge/sessions" / self.session / "owner.json"
        owner = read(owner_path)
        save(self.dir / "owner.json", owner)
        binding = owner["registration"]
        need(not owner["keeper"] and not owner["inspect"]
             and binding["metadata"]["class_id"] == fixture["class_id"], "consumer_owner_class")
        for key in ("environment", "module", "host", "host_source_sha256", "compatibility"):
            need(binding[key] == self.registration[key], "consumer_launch_binding")
        run.capacity(1)
        self.assert_processing()
        run.event("processing", run=label, class_id=fixture["class_id"])

    def drain(self, stream, name, parse):
        total = 0
        try:
            with (self.dir / (name + ".log")).open("wb") as sink:
                for line in iter(stream.readline, b""):
                    total += len(line)
                    need(total <= 16 * 1024 * 1024, "consumer_output_extent")
                    sink.write(line)
                    sink.flush()
                    if parse and line.strip():
                        self.events.append((time.monotonic_ns(), json.loads(line)))
        except Exception as error:
            self.errors.append(error)

    def assert_processing(self):
        need(not self.errors and self.child.poll() is None, "consumer_no_longer_live")
        need(not any(row.get("event") in ("audio", "consumer_timing", "host_retired", "failed") for _, row in self.events), "processing_window_ended")
        need(self.session in self.r.leases(), "processing_lease_missing")

    def finish(self):
        remaining = self.deadline - time.monotonic()
        need(remaining > 0, "consumer_lifecycle_deadline")
        self.child.wait(timeout=remaining)
        for thread in self.threads:
            thread.join(timeout=5)
        need(not self.errors and not any(thread.is_alive() for thread in self.threads), "consumer_output_failed")
        save(self.dir / "events.json", [{"received_ns": at, **row} for at, row in self.events])
        rows = [row for _, row in self.events]
        need(self.child.returncode == 0 and rows[-1]["event"] == "passed", "consumer_lifecycle_failed")
        audio, = [row for row in rows if row.get("event") == "audio"]
        need(audio["callback_audited"] and audio["mismatches"] == audio["nonfinite"] == audio["rejected_callbacks"] == 0
             and audio["nonzero"] > 0 and audio["blocks"] == self.blocks
             and audio["frames_per_callback"] == HOST_FRAMES
             and audio["compared_samples"] == self.blocks * HOST_FRAMES * 2, "consumer_audio_failed")
        need(any(row.get("event") == "host_retired" and row.get("module_unloaded") for row in rows), "consumer_sdk_retirement_missing")
        timing, = [row for row in rows if row.get("event") == "consumer_timing"]
        need(len(timing["blocks"]) == self.blocks
             and all(row["block"] == index and row["scheduled_ns"] == index * HOST_FRAMES * 1000000000 // SAMPLE_RATE
                     for index, row in enumerate(timing["blocks"])), "consumer_timing_count_or_schedule")
        # C++ steady_clock and Python monotonic both use CLOCK_MONOTONIC on this Linux fixture.
        start = timing["audio_begin_monotonic_ns"]
        end = start + timing["blocks"][-1]["started_ns"]
        for label, began, ended in self.intervals:
            need(start < began <= ended < end, "action_not_within_actual_audio_window")
        report_path = self.r.root / "runtime/results" / ("windows-" + self.session + ".json")
        until = min(self.deadline, time.monotonic() + RETIREMENT_SECONDS)
        report, samples = None, []
        while time.monotonic() < until:
            if report_path.exists():
                report = read(report_path)
                if report.get("cleanup_confirmed") is True and report.get("transport_retired") is True and self.session not in self.r.leases():
                    break
            samples.append(self.r.capacity())
            time.sleep(0.1)
        save(self.dir / "retirement.json", {"report": report, "capacity_samples": samples})
        need(report and report.get("cleanup_confirmed") is True and report.get("transport_retired") is True
             and self.session not in self.r.leases(), "positive_product_retirement_missing")
        launch = applied_launch(self.registration, report)
        save(self.dir / "applied-launch-environment.json", launch)
        native = self.r.root / "runtime/results" / ("native-" + self.session + ".jsonl")
        need(native.stat().st_size <= 131072, "native_phases_extent")
        data = native.read_bytes()
        (self.dir / "native-phases.jsonl").write_bytes(data)
        phases = []
        for line in data.splitlines():
            try:
                row = json.loads(line)
            except (ValueError, UnicodeError):
                continue
            if row.get("event") == "ap7_audio_phase":
                phases.append(row)
        need(phases and phases[-1]["phase"] == "retired", "native_retirement_missing")
        need(all(row.get("processing", {}).get("missing_frames", 0) == 0 for row in phases), "native_processing_gap")
        self.r.capacity(0)
        self.finished = True
        self.r.audio_runs.append(audio["compared_samples"])
        self.r.event("audio_passed", run=self.label, blocks=audio["blocks"], compared_samples=audio["compared_samples"],
                     mismatches=0, callback_overruns=audio["callback_overruns"], cleanup_confirmed=True, transport_retired=True,
                     applied_launch_environment=launch)

    def stop(self):
        if self.child.poll() is None:
            self.child.terminate()
            try:
                self.child.wait(timeout=10)
            except subprocess.TimeoutExpired:
                self.child.kill()
                self.child.wait(timeout=5)
            self.r.event("failed_consumer_stopped", run=self.label, retirement_claim=False)
        if not self.finished:
            for thread in self.threads:
                thread.join(timeout=5)
            save(self.dir / "partial-events.json", [{"received_ns": at, **row} for at, row in self.events])
            session = getattr(self, "session", None)
            if session:
                path = self.r.root / "runtime/results" / ("windows-" + session + ".json")
                if path.exists():
                    save(self.dir / "partial-retirement.json", read(path))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--single-trial", action="store_true", help="Graphics trial only; explicitly a subset")
    parser.add_argument("--example-config", action="store_true")
    parser.add_argument("--resume-from", type=Path, help="Reuse a failed run with passed baseline and prepared first trial; exact bindings required")
    args = parser.parse_args()
    if args.example_config:
        print(json.dumps(EXAMPLE, indent=2))
        return 0
    if not (args.config and args.output):
        parser.error("--config and --output are required")
    os.umask(0o077)
    run = None
    try:
        run = Run(read(args.config), args.output, args.single_trial, args.resume_from)
        run.run()
        return 0
    except BaseException as error:
        if run:
            save(run.out / "failure.json", failure_details(error))
            run.event("failed", code=str(error) if isinstance(error, Failed) else type(error).__name__,
                      product_state_preserved=True, automatic_rollback=False)
        else:
            print(json.dumps({"event": "failed_before_run", "code": type(error).__name__}), flush=True)
        return 1
    finally:
        if run:
            for consumer in run.children:
                try:
                    consumer.stop()
                except Exception as error:
                    save(consumer.dir / "test-consumer-cleanup-error.json", {"reason": str(error)})
                    run.event("test_consumer_cleanup_failed", run=consumer.label, retirement_claim=False)


if __name__ == "__main__":
    raise SystemExit(main())

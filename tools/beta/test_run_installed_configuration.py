"""A fresh product read may repeat once; requests and SDK lifetimes may not."""
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch


SOURCE = Path(__file__).with_name("run_installed_configuration.py")
SPEC = importlib.util.spec_from_file_location("configuration_helper", SOURCE)
helper = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(helper)

REFRESH = b'Error: "operator_state_changed_refresh"\n'
AUDIT_REFRESH = (b'PB0_PHASE [["token_before",2],["current_products",301]]\n'
                 b'PB0_PHASE {"external_cleanup_ms":0,"installer_cohorts":6}\n' + REFRESH)


class Clock:
    def __init__(self):
        self.ns = 100_000_000_000

    def read(self):
        return self.ns

    def advance(self, seconds):
        self.ns += int(seconds * 1_000_000_000)


class AppliedLaunch(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name).resolve()
        self.registration = {"host": {"path": str(self.root / "host.exe"), "sha256": "a" * 64},
                             "environment": {"runner": {}},
                             "compatibility": {"disable_windows_accessibility": False}}

    def report(self, overrides, backend=None):
        return {"graphics_configuration": {"requested_backend": backend, "dll_overrides": overrides,
                "scope": "host_process_and_children", "renderer_observed": False}}

    def paired_runtime(self):
        files = {}
        artifacts = []
        for name in ("lvb-direct-wait.dll", "x86_64-windows/lvb-direct-wait.dll",
                     "x86_64-unix/lvb-direct-wait.so"):
            path = self.root / name
            path.parent.mkdir(exist_ok=True)
            path.write_bytes(name.encode())
            path.chmod(0o444)
            files[name] = helper.sha(path)
            artifacts.append({"path": str(path), "sha256": files[name]})
        path = self.root / "direct-audio-helper.json"
        helper.save(path, {"schema": 1, "abi": 1, "host_sha256": "a" * 64, "files": files})
        path.chmod(0o444)
        artifacts.append({"path": str(path), "sha256": helper.sha(path)})
        helper.save(self.root / "runtime.json", {"host": self.registration["host"],
                                                "direct_audio_helpers": artifacts})

    def test_exact_options_include_plugin_defaults_and_bound_paired_helper(self):
        helper.applied_launch(self.registration, self.report("winebus.sys=d"))
        self.paired_runtime()
        self.registration["compatibility"].update(graphics="wine_d3d11", disable_windows_accessibility=True)
        overrides = "d3d11,dxgi=b;uiautomationcore=;winebus.sys=d;lvb-direct-wait=b"
        helper.applied_launch(self.registration, self.report(overrides, "wine_d3d11"))
        for wrong in ("d3d11,dxgi=b;winebus.sys=d;lvb-direct-wait=b",
                      overrides + ";unexpected=b", overrides.replace("dxgi=b", "dxgi=n,b")):
            with self.subTest(overrides=wrong), self.assertRaises(helper.Failed):
                helper.applied_launch(self.registration, self.report(wrong, "wine_d3d11"))

    def test_observed_report_cannot_authorize_changed_helper_bytes_or_host(self):
        self.paired_runtime()
        report = self.report("winebus.sys=d;lvb-direct-wait=b")
        helper.applied_launch(self.registration, report)
        path = self.root / "lvb-direct-wait.dll"
        original = path.read_bytes()
        path.chmod(0o644)
        path.write_bytes(b"changed")
        path.chmod(0o444)
        with self.assertRaises(helper.Failed):
            helper.applied_launch(self.registration, report)
        path.chmod(0o644)
        path.write_bytes(original)
        path.chmod(0o444)
        helper.applied_launch(self.registration, report)
        self.registration["host"]["sha256"] = "b" * 64
        with self.assertRaises(helper.Failed):
            helper.applied_launch(self.registration, report)


class ProductReadRefresh(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.run = helper.Run.__new__(helper.Run)
        self.run.out = Path(temporary.name)
        self.run.root = self.run.out / "managed"
        self.run.c = {"manager": "/declared/manager"}
        self.run.target = {"environment": "a" * 32, "module_sha256": "b" * 64,
                           "class_id": "c" * 32}
        self.run.sibling = {**self.run.target, "class_id": "d" * 32}
        self.run.counter = 0
        self.command = [self.run.c["manager"], "operator", "product",
                        self.run.target["environment"], self.run.target["module_sha256"],
                        self.run.target["class_id"]]
        self.detail = {"schema": 1, "operator_schema": 18, "state_token": "fresh-token",
                       "current_generation": "fresh-generation"}
        self.clock = Clock()

    def refusal(self, **changes):
        fields = {"returncode": 1, "cmd": self.command, "output": b"", "stderr": AUDIT_REFRESH}
        fields.update(changes)
        return subprocess.CalledProcessError(**fields)

    def success(self, value=None):
        return subprocess.CompletedProcess(self.command, 0,
            stdout=json.dumps(self.detail if value is None else value).encode(), stderr=b"")

    def call_detail(self, behavior):
        with patch.object(helper.subprocess, "run", side_effect=behavior) as child, \
                patch.object(helper.time, "monotonic_ns", side_effect=self.clock.read), \
                patch.object(helper, "Consumer") as consumer:
            value = self.run.detail(self.run.target, "after-keep")
            consumer.assert_not_called()
            return value, child.call_args_list

    def test_transient_refusal_returns_only_fresh_read_and_retains_both_attempts(self):
        value, calls = self.call_detail([self.refusal(), self.success()])
        self.assertEqual(value, self.detail)
        self.assertEqual([call.args[0] for call in calls], [self.command, self.command])
        first = helper.read(self.run.out / "after-keep-product-command-failure.json")
        self.assertEqual(first["stderr"]["text"], AUDIT_REFRESH.decode())
        self.assertEqual(first["returncode"], 1)
        self.assertEqual(helper.read(self.run.out / "after-keep-product-read-2.json"), self.detail)
        self.assertEqual(helper.read(self.run.out / "after-keep-product.json"), self.detail)
        for stem in ("after-keep-product", "after-keep-product-read-2"):
            self.assertTrue((self.run.out / (stem + "-command-interval.json")).exists())

    def test_exact_refusal_without_audit_diagnostics_also_retries(self):
        value, calls = self.call_detail([self.refusal(stderr=REFRESH), self.success()])
        self.assertEqual(value, self.detail)
        self.assertEqual(len(calls), 2)

    def test_repeated_refusal_stops_after_second_read_with_distinct_evidence(self):
        with patch.object(helper.subprocess, "run", side_effect=[self.refusal(), self.refusal()]) as child:
            with self.assertRaises(subprocess.CalledProcessError):
                self.run.detail(self.run.target, "repeated")
        self.assertEqual(child.call_count, 2)
        for stem in ("repeated-product", "repeated-product-read-2"):
            self.assertEqual(helper.read(self.run.out / (stem + "-command-failure.json"))["stderr"]["text"],
                             AUDIT_REFRESH.decode())
        self.assertFalse((self.run.out / "repeated-product.json").exists())

    def test_unrelated_malformed_and_timeout_failures_never_retry(self):
        failures = [
            self.refusal(returncode=2), self.refusal(returncode=-9), self.refusal(returncode=True),
            self.refusal(output=b"{}"), self.refusal(cmd=self.command[:-1] + ["d" * 32]),
            self.refusal(stderr=b'Error: "operator_stale_request_refresh"\n'),
            self.refusal(stderr=b'Error: "operator_state_changed_refresh_extra"\n'),
            self.refusal(stderr=REFRESH + REFRESH),
            self.refusal(stderr=REFRESH + b'Error: "owner_error"\n'),
            self.refusal(stderr=REFRESH + b"unknown diagnostic\n"),
            self.refusal(stderr=REFRESH + b"PB0_PHASE not-json\n"),
            self.refusal(stderr=REFRESH + b"PB0_PHASE false\n"),
            self.refusal(stderr=None), self.refusal(stderr=REFRESH.decode()),
            self.refusal(stderr=REFRESH + b'PB0_PHASE {"invalid":"\xff"}\n'),
            subprocess.TimeoutExpired(self.command, 45, output=b"", stderr=REFRESH),
        ]
        for index, failure in enumerate(failures):
            with self.subTest(failure=failure), \
                    patch.object(helper.subprocess, "run", side_effect=failure) as child:
                with self.assertRaises((subprocess.CalledProcessError, subprocess.TimeoutExpired)):
                    self.run.detail(self.run.target, f"unrelated-{index}")
                self.assertEqual(child.call_count, 1)

    def test_successful_malformed_or_wrong_schema_read_is_not_retried(self):
        for label, response, error in (
            ("malformed", subprocess.CompletedProcess(self.command, 0, stdout=b"{", stderr=b""),
             json.JSONDecodeError),
            ("old-schema", self.success({**self.detail, "operator_schema": 17}), helper.Failed),
        ):
            with self.subTest(label=label), patch.object(helper.subprocess, "run", return_value=response) as child:
                with self.assertRaises(error):
                    self.run.detail(self.run.target, label)
                self.assertEqual(child.call_count, 1)

    def test_retry_uses_remaining_original_deadline(self):
        calls = []
        def child(command, **kwargs):
            calls.append(kwargs["timeout"])
            if len(calls) == 1:
                self.clock.advance(12)
                raise self.refusal()
            self.clock.advance(10)
            return self.success()
        value, _ = self.call_detail(child)
        self.assertEqual(value, self.detail)
        self.assertEqual(calls, [45, 33])
        self.assertEqual(helper.read(self.run.out / "after-keep-product-refresh.json")["deadline_ns"],
                         145_000_000_000)

    def test_exhausted_deadline_does_not_start_second_child(self):
        def child(command, **kwargs):
            self.clock.advance(45)
            raise self.refusal()
        with patch.object(helper.subprocess, "run", side_effect=child) as process, \
                patch.object(helper.time, "monotonic_ns", side_effect=self.clock.read):
            with self.assertRaises(subprocess.TimeoutExpired) as expired:
                self.run.detail(self.run.target, "expired")
        self.assertEqual(process.call_count, 1)
        self.assertEqual(expired.exception.timeout, 45)
        failure = helper.read(self.run.out / "expired-product-read-2-command-failure.json")
        self.assertIs(failure["command_started"], False)
        self.assertEqual(failure["deadline_ns"], 145_000_000_000)

    def test_second_child_timeout_is_not_retried_or_given_fresh_deadline(self):
        timeouts = []
        def child(command, **kwargs):
            timeouts.append(kwargs["timeout"])
            if len(timeouts) == 1:
                self.clock.advance(12)
                raise self.refusal()
            self.clock.advance(kwargs["timeout"])
            raise subprocess.TimeoutExpired(command, kwargs["timeout"])
        with patch.object(helper.subprocess, "run", side_effect=child) as process, \
                patch.object(helper.time, "monotonic_ns", side_effect=self.clock.read):
            with self.assertRaises(subprocess.TimeoutExpired):
                self.run.detail(self.run.target, "timed-out")
        self.assertEqual(process.call_count, 2)
        self.assertEqual(timeouts, [45, 33])
        self.assertEqual(self.clock.ns, 145_000_000_000)

    def test_fresh_read_binds_exactly_one_request_and_does_not_restart_audio(self):
        self.detail["operator_schema"] = 20
        action = {"kind": "buffering_set", "class_id": self.run.target["class_id"], "added_frames": 512}
        self.detail["offers"] = [{"label": "Use 512", "action": action, "disabled_reason": None}]
        operation = "e" * 32
        commands = []
        def child(command, **kwargs):
            commands.append(command)
            if len(commands) == 1:
                raise self.refusal()
            if len(commands) == 2:
                return self.success()
            self.assertEqual(command, [self.run.c["manager"], "operator", "request"])
            request = json.loads(kwargs["input"])
            self.assertEqual(request, {"schema": 20, "state_token": "fresh-token", "action": action})
            directory = self.run.root / "operator" / operation
            directory.mkdir(parents=True)
            helper.save(directory / "request.json", request)
            helper.save(directory / "result.json", {"schema": 1, "operation": operation,
                        "state": "completed", "result": {"added_bridge_frames": 512}})
            receipt = {"schema": 20, "operation": operation, "accepted": True, "refusal": None}
            return subprocess.CompletedProcess(command, 0, stdout=json.dumps(receipt).encode(), stderr=b"")
        with patch.object(helper.subprocess, "run", side_effect=child), \
                patch.object(helper, "Consumer") as consumer, patch.object(self.run, "event"):
            result, selected = self.run.action("buffering", lambda offered: offered == action)
            consumer.assert_not_called()
        self.assertEqual(selected, action)
        self.assertEqual(result, {"added_bridge_frames": 512})
        self.assertEqual(commands, [self.command, self.command,
                                   [self.run.c["manager"], "operator", "request"]])

    def test_request_command_failure_is_fatal_without_resubmission(self):
        action = {"kind": "buffering_set", "class_id": self.run.target["class_id"], "added_frames": 512}
        self.detail["offers"] = [{"label": "Use 512", "action": action, "disabled_reason": None}]
        request_command = [self.run.c["manager"], "operator", "request"]
        with patch.object(helper.subprocess, "run", side_effect=[self.success(), self.refusal(cmd=request_command)]) as child:
            with self.assertRaises(subprocess.CalledProcessError):
                self.run.action("buffering", lambda offered: offered == action)
        self.assertEqual([call.args[0] for call in child.call_args_list], [self.command, request_command])

    def test_other_commands_keep_their_original_single_attempt(self):
        for command in ([self.run.c["manager"], "operator", "request"], ["declared-sdk-host", "sibling"],
                        [self.run.c["manager"], "operator", "overview"]):
            with self.subTest(command=command), \
                    patch.object(helper.subprocess, "run", side_effect=self.refusal(cmd=command)) as child:
                with self.assertRaises(subprocess.CalledProcessError):
                    self.run.command("ordinary", command)
                self.assertEqual(child.call_count, 1)


if __name__ == "__main__":
    unittest.main()

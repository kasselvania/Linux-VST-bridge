"""Operator-facing failure reporting; no network or workload execution."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch

TOOLS = Path(__file__).resolve().parent
sys.path.insert(0, str(TOOLS))
from classified_proof_backend import DiagnosticPlanAdapter, PreflightFailed
from pc0_proof_adapter import PLAN_DESCRIPTOR, PLAN_ID, CommandReply, SubprocessCommandPort, StrictSSHPort

spec = importlib.util.spec_from_file_location("proof_cli_under_test", TOOLS / "proof-run.py")
cli = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cli)


class ProofCliTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.authority = self.root / "campaign.md"
        record = PLAN_DESCRIPTOR.record()
        fields = {
            "status": "active_diagnostic_campaign",
            "authority_phase": "proof_harness_maintenance",
            "change_class": "PROOF_HARNESS_MAINTENANCE",
            "product_implementation_authorized": "false",
            "live_execution_authorized": "true",
            "permitted_execution_class": "DIAGNOSTIC_NON_AUTHORITATIVE",
            "classified_backend_core_ready": "true",
            "classified_backend_ready": "true",
            "authorized_source_commit": "1" * 40,
            "authorized_plan_id": PLAN_ID,
            "authorized_product_contract_identity": record["product_contract_identity"],
            "authorized_product_contract_sha256": record["product_contract_sha256"],
            "authorized_plan_content_sha256": record["plan_content_sha256"],
            "diagnostic_campaign_identity": "2" * 64,
            "diagnostic_batch_budget": "2",
        }
        self.authority.write_text("## Authority\n```yaml\n" + "\n".join(f"{k}: {v}" for k, v in fields.items()) + "\n```\n")
        self.args = ["diagnose", "--authority", str(self.authority), "--source", "1" * 40,
                     "--plan", PLAN_ID, "--campaign", "2" * 64]

    def adapter(self, preflight):
        return DiagnosticPlanAdapter(
            descriptor=PLAN_DESCRIPTOR, preflight=preflight,
            reconcile=Mock(side_effect=AssertionError("reconciliation not expected")),
            invoke=Mock(side_effect=AssertionError("workload forbidden")),
            admit=Mock(side_effect=AssertionError("admission not expected")))

    def test_remote_nonzero_exit_reaches_operator_with_cause_and_no_reservation(self):
        command = cli.ReportingCommandPort()
        ssh = StrictSSHPort(command)
        ssh._destination = "local-test-port"
        adapter = self.adapter(lambda context: ssh.run_python(
            "/unused", "pass", ("inspect", "{}"), timeout=1))
        reply = CommandReply(1, b"", b"Traceback omitted\nRuntimeError: launch-critical size mismatch: steamapps/appmanifest_4183110.acf\n")
        with patch.object(SubprocessCommandPort, "run", return_value=reply), \
             patch.object(cli, "_production_adapters", return_value=({PLAN_ID: adapter}, command)), \
             patch.object(cli, "STATE_ROOT", self.root / "ledger"):
            err = io.StringIO()
            with contextlib.redirect_stderr(err):
                code = cli.main(self.args)
        self.assertEqual(code, 2)
        self.assertIn("PreflightFailed", err.getvalue())
        self.assertIn("ssh exit 1", err.getvalue())
        self.assertIn("appmanifest_4183110.acf", err.getvalue())
        self.assertNotIn("Traceback omitted", err.getvalue())
        self.assertFalse((self.root / "ledger").exists())
        adapter.invoke.assert_not_called()

    def test_preflight_only_cannot_construct_transaction_or_invoke_workload(self):
        preflight = Mock()
        adapter = self.adapter(preflight)
        with patch.object(cli, "_production_adapters", return_value=({PLAN_ID: adapter}, cli.ReportingCommandPort())), \
             patch.object(cli, "ClassifiedProofBackend", side_effect=AssertionError("backend forbidden")):
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                code = cli.main(self.args + ["--preflight-only"])
        self.assertEqual(code, 0)
        self.assertEqual(json.loads(out.getvalue())["reservations_created"], 0)
        preflight.assert_called_once()
        adapter.invoke.assert_not_called()

    def test_scoped_authority_does_not_change_default_circuit_breaker(self):
        args = ["diagnose", "--source", "1" * 40, "--plan", PLAN_ID, "--campaign", "2" * 64]
        with patch.object(cli, "_production_adapters", side_effect=AssertionError("ports forbidden")):
            err = io.StringIO()
            with contextlib.redirect_stderr(err):
                self.assertEqual(cli.main(args), 2)
        self.assertIn("LIVE_EXECUTION_FORBIDDEN", err.getvalue())

    def test_error_details_are_bounded_and_redacted(self):
        text = cli.safe_detail("failure /home/private/file.py token=private ghp_abc123 user@example.org 100.2.3.4")
        for secret in ("/home/private", "private", "ghp_abc123", "user@example.org", "100.2.3.4"):
            self.assertNotIn(secret, text)
        self.assertLessEqual(len(cli.safe_detail("x" * 50000)), 768)
        self.assertEqual(cli.safe_detail(b""), "no error detail returned")

    def test_local_plan_and_validation_do_not_load_execution_permission(self):
        for operation in ("plan", "validate"):
            with self.subTest(operation=operation):
                args = cli.build_parser().parse_args([
                    operation, "--source", "1" * 40, "--plan", PLAN_ID])
                local = Mock(return_value=0)
                with patch.object(cli, "load_authority", side_effect=AssertionError("not a live operation")), \
                     patch.object(cli, "_production_adapters", side_effect=AssertionError("no remote adapters")), \
                     patch.object(cli, "ClassifiedProofBackend", side_effect=AssertionError("no transaction")):
                    self.assertEqual(cli.dispatch(args, local_runner=local), 0)
                local.assert_called_once_with(operation, "1" * 40, PLAN_ID)

    def test_legacy_status_is_not_current_task_authority(self):
        self.assertEqual(cli.AUTHORITY, TOOLS / "legacy-proof-default.md")
        with patch.object(cli, "_production_adapters", side_effect=AssertionError("status cannot launch")):
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                self.assertEqual(cli.main(["status"]), 0)
        result = json.loads(out.getvalue())
        self.assertEqual(result["interface"], "legacy_proof_transactions")
        self.assertFalse(result["live_execution_authorized"])
        self.assertIn("CURRENT_SLICE.md", result["project_task_status"])

    def test_legacy_default_does_not_read_the_current_work_document(self):
        original = Path.read_bytes
        def read(path):
            if path == TOOLS.parent / "CURRENT_SLICE.md":
                raise AssertionError("task prose is not a legacy permission source")
            return original(path)
        with patch.object(Path, "read_bytes", read), contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(cli.main(["status"]), 0)


if __name__ == "__main__":
    unittest.main()

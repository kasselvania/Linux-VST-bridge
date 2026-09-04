#!/usr/bin/env python3
"""Deterministic tests for proof-execution classification and hard gating."""

from __future__ import annotations

import argparse
import importlib.util
import pathlib
import subprocess
import sys
import tempfile
import unittest
import unittest.mock


TOOLS = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(TOOLS))

from proof_execution_policy import (  # noqa: E402
    ExecutionClass, LiveRequest, PolicyError, authorize_live_request,
    authority_status, load_authority, validate_delegation,
)


SOURCE = "1" * 40
IDENTITY = "2" * 64
OTHER_IDENTITY = "3" * 64
PLAN = "pc0-pre-setup-processing-contract-v1"


def authority_text(
    *,
    live: bool,
    execution_class: str = "none",
    extra: dict[str, str] | None = None,
) -> str:
    fields = {
        "status": "test",
        "authority_phase": "proof_harness_maintenance",
        "change_class": "PROOF_HARNESS_MAINTENANCE",
        "product_implementation_authorized": "false",
        "live_execution_authorized": "true" if live else "false",
        "permitted_execution_class": execution_class,
    }
    fields.update(extra or {})
    body = "\n".join(f"{key}: {value}" for key, value in fields.items())
    return f"# Current Work\n\n## Authority\n\n```yaml\n{body}\n```\n"


class PolicyTests(unittest.TestCase):
    def load(self, text: str):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        path = pathlib.Path(temporary.name) / "CURRENT_SLICE.md"
        path.write_text(text, encoding="utf-8")
        return load_authority(path)

    def test_current_repair_posture_disables_live_execution(self):
        authority = self.load(authority_text(live=False))
        status = authority_status(authority)
        self.assertFalse(status["live_execution_authorized"])
        with self.assertRaisesRegex(PolicyError, "LIVE_EXECUTION_FORBIDDEN"):
            authorize_live_request(
                authority,
                LiveRequest(
                    ExecutionClass.ACCEPTANCE_CANDIDATE,
                    SOURCE, PLAN, IDENTITY,
                ),
            )

    def test_diagnostic_authority_is_permanently_ineligible(self):
        authority = self.load(authority_text(
            live=True,
            execution_class="DIAGNOSTIC_NON_AUTHORITATIVE",
            extra={
                "classified_backend_ready": "true",
                "authorized_source_commit": SOURCE,
                "authorized_plan_id": PLAN,
                "diagnostic_campaign_identity": IDENTITY,
                "diagnostic_batch_budget": "2",
            },
        ))
        value = authorize_live_request(
            authority,
            LiveRequest(
                ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE,
                SOURCE, PLAN, IDENTITY,
            ),
        )
        self.assertFalse(value["acceptance_eligible"])
        self.assertEqual(value["batch_budget_maximum"], 2)
        self.assertEqual(validate_delegation(authority, value), value)

    def test_acceptance_is_one_exact_candidate(self):
        authority = self.load(authority_text(
            live=True,
            execution_class="ACCEPTANCE_CANDIDATE",
            extra={
                "classified_backend_ready": "true",
                "authorized_source_commit": SOURCE,
                "authorized_plan_id": PLAN,
                "acceptance_candidate_identity": IDENTITY,
                "acceptance_batch_budget": "1",
            },
        ))
        value = authorize_live_request(
            authority,
            LiveRequest(
                ExecutionClass.ACCEPTANCE_CANDIDATE,
                SOURCE, PLAN, IDENTITY,
            ),
        )
        self.assertTrue(value["acceptance_eligible"])
        with self.assertRaises(PolicyError):
            authorize_live_request(
                authority,
                LiveRequest(
                    ExecutionClass.ACCEPTANCE_CANDIDATE,
                    SOURCE, PLAN, OTHER_IDENTITY,
                ),
            )

    def test_missing_classified_backend_is_fail_closed(self):
        authority = self.load(authority_text(
            live=True,
            execution_class="ACCEPTANCE_CANDIDATE",
            extra={
                "authorized_source_commit": SOURCE,
                "authorized_plan_id": PLAN,
                "acceptance_candidate_identity": IDENTITY,
                "acceptance_batch_budget": "1",
            },
        ))
        with self.assertRaisesRegex(PolicyError, "CLASSIFIED_BACKEND_REQUIRED"):
            authorize_live_request(
                authority,
                LiveRequest(
                    ExecutionClass.ACCEPTANCE_CANDIDATE,
                    SOURCE, PLAN, IDENTITY,
                ),
            )

    def test_malformed_and_duplicate_authority_is_rejected(self):
        duplicate = authority_text(live=False).replace(
            "live_execution_authorized: false",
            "live_execution_authorized: false\nlive_execution_authorized: true",
        )
        with self.assertRaisesRegex(PolicyError, "duplicate authority key"):
            self.load(duplicate)

    def test_direct_legacy_live_entry_is_blocked_before_git_access(self):
        wrapper = TOOLS / "host-proof.py"
        result = subprocess.run(
            [sys.executable, str(wrapper), "run",
             "--source", SOURCE, "--plan", PLAN],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("UNCLASSIFIED_LIVE_EXECUTION_FORBIDDEN", result.stderr)

    def test_proof_run_never_calls_backend_for_live_request(self):
        path = TOOLS / "proof-run.py"
        spec = importlib.util.spec_from_file_location("proof_run", path)
        assert spec and spec.loader
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        calls: list[tuple[str, str, str]] = []
        args = argparse.Namespace(
            operation="accept", source=SOURCE, plan=PLAN, candidate=IDENTITY,
        )
        with unittest.mock.patch.object(module, "AUTHORITY", self._write_disabled()):
            with self.assertRaisesRegex(PolicyError, "LIVE_EXECUTION_FORBIDDEN"):
                module.dispatch(args, local_runner=lambda *value: calls.append(value) or 0)
        self.assertEqual(calls, [])

    def _write_disabled(self) -> pathlib.Path:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        path = pathlib.Path(temporary.name) / "CURRENT_SLICE.md"
        path.write_text(authority_text(live=False), encoding="utf-8")
        return path


if __name__ == "__main__":
    unittest.main(verbosity=2)

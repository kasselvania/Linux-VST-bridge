#!/usr/bin/env python3
"""Deterministic tests for PX2 authority, delegation, and circuit breakers."""

from __future__ import annotations

import argparse
import importlib.util
import json
import os
import pathlib
import pwd
import subprocess
import sys
import tempfile
import unittest
import unittest.mock


TOOLS = pathlib.Path(__file__).resolve().parent
ROOT = TOOLS.parent
sys.path.insert(0, str(TOOLS))

from proof_execution_policy import (  # noqa: E402
    ExecutionClass,
    LiveRequest,
    PolicyError,
    authority_status,
    authorize_live_request,
    canonical_json,
    load_authority,
    validate_canonical_delegation,
    validate_delegation,
)


SOURCE = "1" * 40
IDENTITY = "2" * 64
OTHER_IDENTITY = "3" * 64
PLAN = "px2-test-plan"
CONTRACT_ID = "pc0-selection-v2"
CONTRACT_SHA = "4" * 64
PLAN_SHA = "5" * 64


def authority_text(
    *,
    mode: str,
    source: str = SOURCE,
    identity: str = IDENTITY,
    budget: int = 1,
    overrides: dict[str, str] | None = None,
) -> str:
    if mode in {"maintenance", "complete"}:
        fields = {
            "status": (
                "no_active_slice" if mode == "complete"
                else "active_proof_harness_maintenance"
            ),
            "authority_phase": (
                "no_active_slice" if mode == "complete"
                else "proof_harness_maintenance"
            ),
            "change_class": "PROOF_HARNESS_MAINTENANCE",
            "maintenance_implementation_authorized": (
                "false" if mode == "complete" else "true"
            ),
            "product_implementation_authorized": "false",
            "live_execution_authorized": "false",
            "permitted_execution_class": "none",
            "classified_backend_core_ready": "true",
            "classified_backend_ready": "false",
        }
    elif mode == "diagnostic":
        fields = {
            "status": "active_diagnostic_campaign",
            "authority_phase": "proof_harness_maintenance",
            "change_class": "PROOF_HARNESS_MAINTENANCE",
            "product_implementation_authorized": "false",
            "live_execution_authorized": "true",
            "permitted_execution_class": "DIAGNOSTIC_NON_AUTHORITATIVE",
            "classified_backend_core_ready": "true",
            "classified_backend_ready": "true",
            "authorized_source_commit": source,
            "authorized_plan_id": PLAN,
            "authorized_product_contract_identity": CONTRACT_ID,
            "authorized_product_contract_sha256": CONTRACT_SHA,
            "authorized_plan_content_sha256": PLAN_SHA,
            "diagnostic_campaign_identity": identity,
            "diagnostic_batch_budget": str(budget),
        }
    elif mode == "acceptance":
        fields = {
            "status": "active_acceptance_candidate",
            "authority_phase": "implementation",
            "change_class": "PRODUCT_CONTRACT_CHANGE",
            "product_implementation_authorized": "true",
            "live_execution_authorized": "true",
            "permitted_execution_class": "ACCEPTANCE_CANDIDATE",
            "classified_backend_core_ready": "true",
            "classified_backend_ready": "true",
            "authorized_source_commit": source,
            "authorized_plan_id": PLAN,
            "authorized_product_contract_identity": CONTRACT_ID,
            "authorized_product_contract_sha256": CONTRACT_SHA,
            "authorized_plan_content_sha256": PLAN_SHA,
            "acceptance_candidate_identity": identity,
            "acceptance_batch_budget": str(budget),
        }
    else:
        raise AssertionError(mode)
    fields.update(overrides or {})
    body = "\n".join(f"{key}: {value}" for key, value in fields.items())
    return f"# Current Work\n\n## Authority\n\n```yaml\n{body}\n```\n"


class PolicyTests(unittest.TestCase):
    def load(self, text: str):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        path = pathlib.Path(temporary.name) / "CURRENT_SLICE.md"
        path.write_text(text, encoding="utf-8")
        return load_authority(path)

    def delegation(self, mode: str, **kwargs):
        authority = self.load(authority_text(mode=mode, **kwargs))
        execution_class = (
            ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE
            if mode == "diagnostic" else ExecutionClass.ACCEPTANCE_CANDIDATE
        )
        value = authorize_live_request(
            authority,
            LiveRequest(execution_class, kwargs.get("source", SOURCE), PLAN,
                        kwargs.get("identity", IDENTITY)),
        )
        return authority, value

    def test_current_authority_disables_every_live_execution(self):
        authority = load_authority(ROOT / "CURRENT_SLICE.md")
        status = authority_status(authority)
        self.assertFalse(status["live_execution_authorized"])
        self.assertTrue(status["classified_backend_core_ready"])
        self.assertTrue(status["classified_backend_ready"])
        self.assertEqual(status["status"], "no_active_slice")
        self.assertEqual(status["authority_phase"], "no_active_slice")
        self.assertFalse(authority.boolean("maintenance_implementation_authorized"))
        self.assertEqual(authority.fields["accepted_product_frontier"], "AP1")
        self.assertEqual(
            authority.fields["production_adapter_registry"], "pc0_ap0_ap1_diagnostic_and_acceptance",
        )
        self.assertNotEqual(authority.fields.get("diagnostic_campaign_authorized"), "true")
        self.assertNotEqual(authority.fields.get("acceptance_candidate_authorized"), "true")
        with self.assertRaisesRegex(PolicyError, "LIVE_EXECUTION_FORBIDDEN"):
            authorize_live_request(
                authority,
                LiveRequest(ExecutionClass.ACCEPTANCE_CANDIDATE,
                            SOURCE, PLAN, IDENTITY),
            )

    def test_closed_authority_table_rejects_illegal_combinations(self):
        mutations = {
            "status": "invented_status",
            "authority_phase": "invented_phase",
            "change_class": "PRODUCT_IMPLEMENTATION",
            "product_implementation_authorized": "true",
            "live_execution_authorized": "true",
            "permitted_execution_class": "ACCEPTANCE_CANDIDATE",
            "classified_backend_ready": "true",
        }
        for key, value in mutations.items():
            with self.subTest(key=key):
                authority = self.load(authority_text(
                    mode="maintenance", overrides={key: value},
                ))
                with self.assertRaisesRegex(PolicyError, "closed PX2 table"):
                    authority_status(authority)

    def test_unknown_class_and_malformed_boolean_fail_closed(self):
        for overrides in (
            {"permitted_execution_class": "UNKNOWN_CLASS"},
            {"live_execution_authorized": "yes"},
        ):
            authority = self.load(authority_text(
                mode="maintenance", overrides=overrides,
            ))
            with self.assertRaises(PolicyError):
                authority_status(authority)

    def test_diagnostic_delegation_is_exact_and_permanently_ineligible(self):
        authority, value = self.delegation("diagnostic", budget=2)
        self.assertFalse(value["acceptance_eligible"])
        self.assertEqual(value["batch_budget_maximum"], 2)
        self.assertEqual(value["product_contract_sha256"], CONTRACT_SHA)
        self.assertEqual(value["plan_content_sha256"], PLAN_SHA)
        self.assertEqual(validate_delegation(authority, value), value)
        self.assertEqual(
            validate_canonical_delegation(authority, canonical_json(value)), value,
        )

    def test_explicit_finite_diagnostic_ceiling_does_not_change_acceptance(self):
        for maximum in (1, 2, 8, 11):
            authority, value = self.delegation("diagnostic", budget=maximum)
            self.assertEqual(value["batch_budget_maximum"], maximum)
            self.assertFalse(value["acceptance_eligible"])
            self.assertEqual(validate_delegation(authority, value), value)
            with self.assertRaises(PolicyError):
                validate_delegation(authority, {**value, "acceptance_eligible":True})
        for maximum in (0, -1, True, "unlimited", "1.5"):
            with self.subTest(maximum=maximum), self.assertRaises(PolicyError):
                self.delegation("diagnostic", budget=maximum)
        with self.assertRaises(PolicyError):
            self.delegation("acceptance", budget=8)

    def test_acceptance_is_one_exact_candidate(self):
        authority, value = self.delegation("acceptance")
        self.assertTrue(value["acceptance_eligible"])
        self.assertEqual(value["batch_budget_maximum"], 1)
        with self.assertRaises(PolicyError):
            authorize_live_request(
                authority,
                LiveRequest(ExecutionClass.ACCEPTANCE_CANDIDATE,
                            SOURCE, PLAN, OTHER_IDENTITY),
            )

    def test_acceptance_uses_only_canonical_product_authority_vocabulary(self):
        authority, _value = self.delegation("acceptance")
        status = authority_status(authority)
        self.assertEqual(status["change_class"], "PRODUCT_CONTRACT_CHANGE")
        self.assertEqual(status["authority_phase"], "implementation")
        for overrides in (
            {"change_class": "PRODUCT_IMPLEMENTATION"},
            {"authority_phase": "product_acceptance"},
        ):
            with self.subTest(overrides=overrides):
                invalid = self.load(authority_text(
                    mode="acceptance", overrides=overrides,
                ))
                with self.assertRaisesRegex(PolicyError, "closed PX2 table"):
                    authority_status(invalid)

    def test_no_active_core_ready_posture_is_accepted_by_status_command(self):
        complete = self.load(authority_text(
            mode="complete", overrides={"classified_backend_ready": "true"},
        ))
        status = authority_status(complete)
        self.assertEqual(status["authority_phase"], "no_active_slice")
        self.assertTrue(status["classified_backend_core_ready"])
        self.assertTrue(status["classified_backend_ready"])
        self.assertFalse(status["live_execution_authorized"])
        local = self.load(authority_text(mode="maintenance"))
        self.assertEqual(authority_status(local)["authority_phase"],
                         "proof_harness_maintenance")
        result = subprocess.run(
            [sys.executable, str(TOOLS / "proof-run.py"), "status"],
            stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        command_status = json.loads(result.stdout)
        self.assertEqual(command_status["status"], "no_active_slice")
        self.assertEqual(command_status["authority_phase"], "no_active_slice")
        self.assertTrue(command_status["classified_backend_core_ready"])
        self.assertTrue(command_status["classified_backend_ready"])
        self.assertFalse(command_status["live_execution_authorized"])
        self.assertEqual(command_status["permitted_execution_class"], "none")

    def test_production_state_root_ignores_home_and_xdg_environment(self):
        real_home = pathlib.Path(pwd.getpwuid(os.getuid()).pw_dir)
        roots = []
        for suffix in ("first", "second"):
            with unittest.mock.patch.dict(os.environ, {
                "HOME": f"/caller-controlled/{suffix}/home",
                "XDG_STATE_HOME": f"/caller-controlled/{suffix}/state",
            }):
                module = self._proof_run_module()
                roots.append(module.STATE_ROOT)
                with unittest.mock.patch.object(module.sys, "platform", "darwin"):
                    self.assertEqual(
                        module.production_state_root(),
                        real_home / "Library" / "Application Support"
                        / "Linux VST Bridge" / "proof" / "classified-proof",
                    )
                with unittest.mock.patch.object(module.sys, "platform", "linux"):
                    self.assertEqual(
                        module.production_state_root(),
                        real_home / ".local" / "state" / "linux-vst-bridge"
                        / "classified-proof",
                    )
        self.assertEqual(roots[0], roots[1])

    def test_stale_tampered_or_noncanonical_delegation_is_rejected(self):
        authority, value = self.delegation("diagnostic")
        tampered = dict(value)
        tampered["plan_content_sha256"] = "9" * 64
        with self.assertRaisesRegex(PolicyError, "differs from current authority"):
            validate_delegation(authority, tampered)

        stale_authority = self.load(authority_text(
            mode="diagnostic", source="a" * 40,
        ))
        with self.assertRaises(PolicyError):
            validate_delegation(stale_authority, value)

        noncanonical = ("{\n  \"schema\": \"" + value["schema"] + "\"\n}\n").encode()
        with self.assertRaisesRegex(PolicyError, "not canonical"):
            validate_canonical_delegation(authority, noncanonical)

    def test_duplicate_or_missing_authority_field_is_rejected(self):
        duplicate = authority_text(mode="maintenance").replace(
            "live_execution_authorized: false",
            "live_execution_authorized: false\nlive_execution_authorized: true",
        )
        with self.assertRaisesRegex(PolicyError, "duplicate authority key"):
            self.load(duplicate)
        missing = authority_text(mode="maintenance").replace(
            "classified_backend_ready: false\n", "",
        )
        with self.assertRaisesRegex(PolicyError, "missing keys"):
            self.load(missing)

    def test_direct_legacy_live_operations_are_both_blocked(self):
        wrapper = TOOLS / "host-proof.py"
        for operation in ("run", "seed-fixture"):
            with self.subTest(operation=operation):
                result = subprocess.run(
                    [sys.executable, str(wrapper), operation,
                     "--source", SOURCE, "--plan", PLAN],
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    text=True,
                    check=False,
                )
                self.assertEqual(result.returncode, 2)
                self.assertIn("UNCLASSIFIED_LIVE_EXECUTION_FORBIDDEN", result.stderr)
                self.assertNotIn("LOCAL_PROOF_BACKEND_BLOCKED", result.stderr)

    def test_proof_run_does_not_call_any_backend_when_live_is_disabled(self):
        module = self._proof_run_module()
        calls: list[object] = []
        args = argparse.Namespace(
            operation="accept", source=SOURCE, plan=PLAN, candidate=IDENTITY,
        )
        disabled = self._write(authority_text(mode="maintenance"))
        with unittest.mock.patch.object(module, "AUTHORITY", disabled):
            with self.assertRaisesRegex(PolicyError, "LIVE_EXECUTION_FORBIDDEN"):
                module.dispatch(
                    args,
                    local_runner=lambda *_: 0,
                    classified_runner=lambda *value: calls.append(value),
                )
        self.assertEqual(calls, [])

    def _proof_run_module(self):
        path = TOOLS / "proof-run.py"
        spec = importlib.util.spec_from_file_location("proof_run", path)
        assert spec and spec.loader
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def _write(self, text: str) -> pathlib.Path:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        path = pathlib.Path(temporary.name) / "CURRENT_SLICE.md"
        path.write_text(text, encoding="utf-8")
        return path


if __name__ == "__main__":
    unittest.main(verbosity=2)

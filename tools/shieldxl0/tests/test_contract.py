import hashlib
import json
import unittest
from pathlib import Path


ROOT = Path(__file__).parents[1]
REPO = ROOT.parents[1]


class ContractTest(unittest.TestCase):
    def test_pending_contract_does_not_claim_physical_acceptance(self):
        contract = json.loads((REPO / "evidence/shieldxl0/hardware-contract.json").read_text())
        self.assertEqual(contract["acceptance_status"], "pending_physical_fixture")
        self.assertIsNone(contract["pi"]["revision"])
        self.assertIsNone(contract["audio"]["physical_loopback"])
        self.assertFalse(contract["provisioning"]["physical_verification_completed"])

    def test_retained_overlay_hashes_match_manifest(self):
        manifest = json.loads((ROOT / "pinned-inputs.json").read_text())
        overlay = manifest["adapted_overlay"]
        for name, expected in (
            ("shieldxl0-overlay.dts", overlay["source_sha256"]),
            ("shieldxl0.dtbo", overlay["binary_sha256"]),
        ):
            observed = hashlib.sha256((ROOT / "overlays" / name).read_bytes()).hexdigest()
            self.assertEqual(observed, expected)

    def test_pinned_kernel_gap_is_truthfully_retained(self):
        evidence = json.loads((REPO / "evidence/shieldxl0/kernel-driver-admission.json").read_text())
        self.assertEqual(evidence["config_results"]["CONFIG_SND_SOC_CS4270"], "not set")
        self.assertFalse(evidence["decision"]["custom_kernel_image"])


if __name__ == "__main__":
    unittest.main()

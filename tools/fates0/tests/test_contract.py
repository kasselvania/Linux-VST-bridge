import hashlib
import json
import unittest
from pathlib import Path


ROOT = Path(__file__).parents[1]
REPO = ROOT.parents[1]


class ContractTest(unittest.TestCase):
    def test_pending_contract_does_not_claim_physical_acceptance(self):
        contract = json.loads((REPO / "evidence/fates0/hardware-contract.json").read_text())
        self.assertEqual(contract["acceptance_status"], "pending_physical_fixture")
        self.assertEqual(contract["fixture"]["pi_model"], "Raspberry Pi 5 Model B")
        self.assertEqual(contract["fixture"]["fates_revision"], "v1.8.1")
        self.assertIsNone(contract["fixture"]["pi_revision"])
        self.assertIsNone(contract["audio"]["physical_loopback"])
        self.assertFalse(contract["provisioning"]["physical_verification_completed"])

    def test_retained_overlay_hashes_match_manifest(self):
        manifest = json.loads((ROOT / "pinned-inputs.json").read_text())
        overlay = manifest["original_overlay"]
        for key, path_key in (("source_sha256", "source"), ("binary_sha256", "binary")):
            path = ROOT / overlay[path_key]
            self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), overlay[key])

    def test_distribution_audio_path_is_pinned_without_custom_kernel(self):
        evidence = json.loads((REPO / "evidence/fates0/kernel-driver-admission.json").read_text())
        self.assertEqual(evidence["machine_driver"], "snd-soc-rpi-proto")
        self.assertEqual(evidence["codec"], "WM8731")
        self.assertTrue(evidence["decision"]["distribution_kernel_retained"])
        self.assertFalse(evidence["decision"]["custom_kernel_image"])

    def test_pi5_overlay_is_rp1_specific_and_static_application_passes(self):
        source = (ROOT / "overlays/fates0-overlay.dts").read_text()
        self.assertIn('compatible = "brcm,bcm2712"', source)
        self.assertIn("<&rp1_gpio", source)
        self.assertIn("target = <&rp1_spi0>", source)
        self.assertNotIn('compatible = "wlf,wm8731"', source.lower())
        evidence = json.loads((REPO / "evidence/fates0/overlay-base-admission.json").read_text())
        self.assertEqual(evidence["result"], "pass")
        self.assertFalse(evidence["physical_boot_completed"])

    def test_gpl_oled_provenance_is_exactly_retained(self):
        manifest = json.loads((ROOT / "pinned-inputs.json").read_text())
        prior = manifest["monome_oled_prior_art"]
        self.assertEqual(prior["commit"], "458e2253667a09bd01f1d50cc2b4063c14c491ec")
        service = (ROOT / "oled_service.py").read_text()
        self.assertIn("SPDX-License-Identifier: GPL-2.0-only", service)
        self.assertTrue((ROOT / "licenses/GPL-2.0-only.txt").is_file())

    def test_exact_added_package_hashes_match_provisioning(self):
        manifest = json.loads((ROOT / "pinned-inputs.json").read_text())
        provisioning = (ROOT / "provision.sh").read_text()
        for name, values in manifest["packages_added_by_provisioning"].items():
            self.assertEqual(len(values["sha256"]), 64, name)
            self.assertIn(values["version"], provisioning)
            self.assertIn(values["sha256"], provisioning)


if __name__ == "__main__":
    unittest.main()

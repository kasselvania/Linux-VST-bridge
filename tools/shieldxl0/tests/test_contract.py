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
        self.assertEqual(contract["pi"]["required_model"], "Raspberry Pi 5 Model B")
        self.assertEqual(contract["pi"]["required_ram_bytes"], 8 * 1024**3)
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
        self.assertEqual(evidence["inspected_kernel_package"]["name"], "linux-image-6.18.50+rpt-rpi-2712")
        self.assertEqual(evidence["config_results"]["CONFIG_ARM64_16K_PAGES"], "y")
        self.assertEqual(evidence["config_results"]["CONFIG_SND_DESIGNWARE_I2S"], "m")
        self.assertEqual(evidence["config_results"]["CONFIG_SND_SOC_CS4270"], "not set")
        self.assertFalse(evidence["decision"]["custom_kernel_image"])

    def test_pi5_overlay_uses_rp1_clock_consumer_and_static_application_passes(self):
        source = (ROOT / "overlays/shieldxl0-overlay.dts").read_text()
        self.assertIn('compatible = "brcm,bcm2712"', source)
        self.assertIn("<&i2s_clk_consumer>", source)
        self.assertIn("<&rp1_gpio", source)
        self.assertIn("reset-gpios = <&rp1_gpio 17 0>", source)
        evidence = json.loads((REPO / "evidence/shieldxl0/overlay-base-admission.json").read_text())
        self.assertEqual(evidence["application"]["result"], "pass")
        self.assertIn("i2s_clk_consumer", evidence["application"]["clock_direction"])

    def test_exact_module_build_prerequisites_are_pinned(self):
        manifest = json.loads((ROOT / "pinned-inputs.json").read_text())
        packages = manifest["image_packages_used_without_addition"]
        self.assertEqual(packages["linux-headers-6.18.50+rpt-rpi-2712"], "1:6.18.50-1+rpt1")
        self.assertEqual(packages["build-essential"], "12.12")
        self.assertEqual(packages["gcc"], "4:14.2.0-1")
        self.assertEqual(packages["make"], "4.4.1-2")
        self.assertEqual(packages["binutils"], "2.44-3")

    def test_wrong_board_refusal_is_retained_without_claiming_acceptance(self):
        evidence = json.loads((REPO / "evidence/shieldxl0/fixture-admission-failure.json").read_text())
        self.assertEqual(evidence["result"], "refused_wrong_board")
        self.assertEqual(evidence["required"]["model"], "Raspberry Pi 5 Model B")
        self.assertEqual(evidence["observed"]["board_revision"], "a020d3")
        self.assertFalse(evidence["changes"]["provisioning_started"])


if __name__ == "__main__":
    unittest.main()

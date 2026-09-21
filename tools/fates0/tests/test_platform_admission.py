import os
import pathlib
import stat
import subprocess
import tempfile
import unittest


ROOT = pathlib.Path(__file__).parents[1]
LIB = ROOT / "lib.sh"


class PlatformAdmissionTest(unittest.TestCase):
    def run_function(self, body, *arguments, path=None):
        return subprocess.run(
            ["/bin/bash", "-c", f'source "$1"; {body}', "fates0-test", str(LIB), *arguments],
            env={**os.environ, **({"PATH": path} if path else {})},
            capture_output=True,
            text=True,
        )

    def test_only_v181_with_pi_power_is_admitted(self):
        accepted = self.run_function('require_fixture_declaration "$2" "$3"', "v1.8.1", "pi")
        self.assertEqual(accepted.returncode, 0, accepted.stderr)
        wrong_revision = self.run_function('require_fixture_declaration "$2" "$3"', "v1.7", "pi")
        self.assertNotEqual(wrong_revision.returncode, 0)
        self.assertIn("unsupported Fates revision", wrong_revision.stderr)
        wrong_power = self.run_function('require_fixture_declaration "$2" "$3"', "v1.8.1", "fates-usb-c")
        self.assertNotEqual(wrong_power.returncode, 0)
        self.assertIn("Fates USB-C must remain disconnected", wrong_power.stderr)

    def test_absent_prohibited_runtimes_return_success(self):
        result = self.run_function("refuse_prohibited_runtimes", path="/usr/bin:/bin")
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_present_prohibited_runtime_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            wine = pathlib.Path(directory) / "wine"
            wine.write_text("#!/bin/sh\nexit 0\n")
            wine.chmod(wine.stat().st_mode | stat.S_IXUSR)
            result = self.run_function(
                "refuse_prohibited_runtimes", path=f"{directory}:/usr/bin:/bin"
            )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("prohibited software 'wine' is present", result.stderr)

    def run_boot_check(self, content):
        with tempfile.NamedTemporaryFile(mode="w", delete=False) as stream:
            stream.write(content)
            config = stream.name
        try:
            return self.run_function('refuse_boot_conflicts "$2"', config)
        finally:
            pathlib.Path(config).unlink()

    def test_pinned_image_default_overlays_are_admitted_by_section(self):
        result = self.run_boot_check(
            "dtoverlay=vc4-kms-v3d\n"
            "[cm5]\n"
            "dtoverlay=dwc2,dr_mode=host\n"
            "[pi5]\n"
            "dtoverlay=nospi10\n"
            "[all]\n"
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_old_fates_and_shield_overlays_are_refused(self):
        for overlay in ("fates-buttons-encoders", "rpi-proto", "shieldxl0"):
            with self.subTest(overlay=overlay):
                result = self.run_boot_check(f"[all]\ndtoverlay={overlay}\n")
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("unexpected conflicting boot configuration", result.stderr)

    def test_old_managed_includes_are_refused(self):
        for include in ("shieldxl0.conf", "fates.conf"):
            with self.subTest(include=include):
                result = self.run_boot_check(f"include {include}\n")
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("unexpected conflicting boot configuration", result.stderr)

    def test_unknown_overlay_is_refused(self):
        result = self.run_boot_check("[pi5]\ndtoverlay=unexpected\n")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unadmitted existing device-tree overlay", result.stderr)

    def test_i2c_character_device_is_loaded_after_adapter_admission(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            adapter = root / "i2c-1-adapter"
            device = root / "i2c-1-device"
            adapter.touch()
            modprobe = root / "modprobe"
            modprobe.write_text(f'#!/bin/sh\n: >"{device}"\n')
            modprobe.chmod(modprobe.stat().st_mode | stat.S_IXUSR)
            result = self.run_function(
                'ensure_i2c_character_device 1 "$2" "$3"',
                str(adapter),
                str(device),
                path=f"{directory}:/usr/bin:/bin",
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertTrue(device.exists())

    def test_missing_i2c_adapter_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            result = self.run_function(
                'ensure_i2c_character_device 1 "$2" "$3"',
                str(root / "missing-adapter"),
                str(root / "missing-device"),
            )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("I2C adapter 1 is absent", result.stderr)


if __name__ == "__main__":
    unittest.main()

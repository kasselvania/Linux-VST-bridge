import os
import pathlib
import stat
import subprocess
import tempfile
import unittest


ROOT = pathlib.Path(__file__).parents[1]
LIB = ROOT / "lib.sh"


class PlatformAdmissionTest(unittest.TestCase):
    def run_refusal_check(self, path):
        return subprocess.run(
            [
                "/bin/bash",
                "-c",
                'source "$1"; refuse_prohibited_runtimes',
                "test-platform-admission",
                str(LIB),
            ],
            env={**os.environ, "PATH": path},
            capture_output=True,
            text=True,
        )

    def test_absent_prohibited_runtimes_return_success(self):
        result = self.run_refusal_check("/usr/bin:/bin")
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_present_prohibited_runtime_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            wine = pathlib.Path(directory) / "wine"
            wine.write_text("#!/bin/sh\nexit 0\n")
            wine.chmod(wine.stat().st_mode | stat.S_IXUSR)
            result = self.run_refusal_check(f"{directory}:/usr/bin:/bin")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("prohibited software 'wine' is present", result.stderr)

    def run_boot_check(self, content):
        with tempfile.NamedTemporaryFile(mode="w", delete=False) as stream:
            stream.write(content)
            config = stream.name
        try:
            return subprocess.run(
                [
                    "/bin/bash",
                    "-c",
                    'source "$1"; refuse_boot_conflicts "$2"',
                    "test-platform-admission",
                    str(LIB),
                    config,
                ],
                capture_output=True,
                text=True,
            )
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

    def test_default_overlay_in_wrong_section_is_refused(self):
        result = self.run_boot_check("[all]\ndtoverlay=dwc2,dr_mode=host\n")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unadmitted existing device-tree overlay", result.stderr)

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
            modprobe.write_text('#!/bin/sh\n: >"$FAKE_I2C_DEVICE"\n')
            modprobe.chmod(modprobe.stat().st_mode | stat.S_IXUSR)
            result = subprocess.run(
                [
                    "/bin/bash",
                    "-c",
                    'source "$1"; ensure_i2c_character_device 1 "$2" "$3"',
                    "test-platform-admission",
                    str(LIB),
                    str(adapter),
                    str(device),
                ],
                env={
                    **os.environ,
                    "PATH": f"{directory}:/usr/bin:/bin",
                    "FAKE_I2C_DEVICE": str(device),
                },
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertTrue(device.exists())

    def test_missing_i2c_adapter_is_refused_before_module_load(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            result = subprocess.run(
                [
                    "/bin/bash",
                    "-c",
                    'source "$1"; ensure_i2c_character_device 1 "$2" "$3"',
                    "test-platform-admission",
                    str(LIB),
                    str(root / "missing-adapter"),
                    str(root / "missing-device"),
                ],
                capture_output=True,
                text=True,
            )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("I2C adapter 1 is absent", result.stderr)

    def test_i2c_dev_persistence_file_is_owned_by_install_and_uninstall(self):
        modules = ROOT / "config" / "shieldxl0-modules.conf"
        self.assertEqual(modules.read_text(), "# SHIELDXL0 I2C userspace interface.\ni2c-dev\n")
        for script in ("provision.sh", "uninstall.sh"):
            source = (ROOT / script).read_text()
            self.assertIn("config/shieldxl0-modules.conf", source)
            self.assertIn("/etc/modules-load.d/shieldxl0.conf", source)


if __name__ == "__main__":
    unittest.main()

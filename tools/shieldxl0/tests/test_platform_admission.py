import os
import pathlib
import stat
import subprocess
import tempfile
import unittest


ROOT = pathlib.Path(__file__).parents[1]
LIB = ROOT / "lib.sh"


class PlatformAdmissionTest(unittest.TestCase):
    def run_install_exact(self, installed_content, predecessor_hash=""):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            source = root / "source"
            destination = root / "destination"
            source.write_text("current\n")
            destination.write_text(installed_content)
            result = subprocess.run(
                [
                    "/bin/bash",
                    "-c",
                    'source "$1"; install_exact "$2" "$3" 0644 "$4"',
                    "test-platform-admission",
                    str(LIB),
                    str(source),
                    str(destination),
                    predecessor_hash,
                ],
                capture_output=True,
                text=True,
            )
            observed = destination.read_text()
        return result, observed

    def test_exact_admitted_predecessor_can_be_upgraded(self):
        import hashlib

        predecessor = "predecessor\n"
        predecessor_hash = hashlib.sha256(predecessor.encode()).hexdigest()
        result, observed = self.run_install_exact(predecessor, predecessor_hash)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(observed, "current\n")

    def test_unrecognized_predecessor_is_refused(self):
        result, observed = self.run_install_exact("foreign\n", "0" * 64)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(observed, "foreign\n")
        self.assertIn("foreign replacement file exists", result.stderr)

    def run_profile_check(self, kernel, page_size):
        return subprocess.run(
            [
                "/bin/bash",
                "-c",
                'source "$1"; select_platform_contract "$2" "$3"; printf "%s\\t%s\\t%s\\t%s\\n" "$SHIELDXL0_PLATFORM_PROFILE" "$SHIELDXL0_KERNEL" "$SHIELDXL0_PAGE_SIZE" "$SHIELDXL0_HEADERS_PACKAGE"',
                "test-platform-admission",
                str(LIB),
                kernel,
                str(page_size),
            ],
            capture_output=True,
            text=True,
        )

    def test_exact_16k_platform_profile_is_admitted(self):
        result = self.run_profile_check("6.18.50+rpt-rpi-2712", 16384)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            result.stdout,
            "shieldxl0-16k\t6.18.50+rpt-rpi-2712\t16384\tlinux-headers-6.18.50+rpt-rpi-2712\n",
        )

    def test_exact_rpi0_4k_platform_profile_is_admitted(self):
        result = self.run_profile_check("6.18.50+rpt-rpi-v8", 4096)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            result.stdout,
            "rpi0-4k-integration\t6.18.50+rpt-rpi-v8\t4096\tlinux-headers-6.18.50+rpt-rpi-v8\n",
        )

    def test_crossed_kernel_page_size_pair_is_refused(self):
        result = self.run_profile_check("6.18.50+rpt-rpi-v8", 16384)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unsupported kernel/page-size pair", result.stderr)

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

    def run_boot_check(self, content, profile="shieldxl0-16k"):
        with tempfile.NamedTemporaryFile(mode="w", delete=False) as stream:
            stream.write(content)
            config = stream.name
        try:
            return subprocess.run(
                [
                    "/bin/bash",
                    "-c",
                    'source "$1"; SHIELDXL0_PLATFORM_PROFILE="$3"; refuse_boot_conflicts "$2"',
                    "test-platform-admission",
                    str(LIB),
                    config,
                    profile,
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

    def test_exact_rpi0_kernel_selector_is_admitted_only_in_4k_profile(self):
        content = "[all]\nkernel=kernel8.img\n"
        result = self.run_boot_check(content, "rpi0-4k-integration")
        self.assertEqual(result.returncode, 0, result.stderr)
        refused = self.run_boot_check(content, "shieldxl0-16k")
        self.assertNotEqual(refused.returncode, 0)
        self.assertIn("unadmitted kernel selector", refused.stderr)

    def test_other_kernel_selector_is_refused_in_4k_profile(self):
        result = self.run_boot_check("[all]\nkernel=foreign.img\n", "rpi0-4k-integration")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unadmitted kernel selector", result.stderr)

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

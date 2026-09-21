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


if __name__ == "__main__":
    unittest.main()

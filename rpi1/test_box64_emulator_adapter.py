import json
import os
import pathlib
import shutil
import subprocess
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parent
SOURCE = ROOT / "box64-emulator-adapter.c"


class Box64EmulatorAdapterTests(unittest.TestCase):
    def setUp(self):
        compiler = shutil.which("cc")
        if compiler is None:
            self.skipTest("cc is unavailable")
        self.temporary = tempfile.TemporaryDirectory()
        root = pathlib.Path(self.temporary.name)
        self.adapter = root / "adapter"
        (root / "bin").mkdir()
        subprocess.run(
            [compiler, "-std=c11", "-Wall", "-Wextra", "-Werror", "-O2", SOURCE, "-o", self.adapter],
            check=True,
        )
        fake = root / "bin/box64"
        fake.write_text(
            "#!/usr/bin/env python3\n"
            "import json, os, sys\n"
            "print(json.dumps({'argv': sys.argv, 'library_path': os.environ.get('BOX64_LD_LIBRARY_PATH')}))\n",
            encoding="utf-8",
        )
        fake.chmod(0o755)

    def tearDown(self):
        self.temporary.cleanup()

    def run_adapter(self, *arguments, env=None):
        completed = subprocess.run(
            [self.adapter, *arguments],
            env=env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        return completed

    def test_direct_target_is_forwarded_unchanged(self):
        completed = self.run_adapter("/fixture/program", "one")
        self.assertEqual(completed.returncode, 0, completed.stderr)
        observed = json.loads(completed.stdout)
        self.assertEqual(observed["argv"][1:], ["/fixture/program", "one"])
        self.assertIsNone(observed["library_path"])

    def test_loader_vector_becomes_box64_library_path(self):
        completed = self.run_adapter(
            "/provider/lib64/ld-linux-x86-64.so.2",
            "--library-path",
            "/provider/lib:/provider/lib64",
            "/fixture/program",
            "two",
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        observed = json.loads(completed.stdout)
        self.assertEqual(observed["argv"][1:], ["/fixture/program", "two"])
        self.assertEqual(observed["library_path"], "/provider/lib:/provider/lib64")

    def test_exact_proton_python_shebang_uses_x86_python(self):
        target = pathlib.Path(self.temporary.name) / "proton"
        target.write_text("#!/usr/bin/env python3\nprint('fixture')\n", encoding="utf-8")
        target.chmod(0o755)
        completed = self.run_adapter(str(target), "getcompatpath", "/")
        self.assertEqual(completed.returncode, 0, completed.stderr)
        observed = json.loads(completed.stdout)
        self.assertEqual(
            observed["argv"][1:],
            ["/usr/bin/python3", str(target), "getcompatpath", "/"],
        )

    def test_non_proton_python_script_is_not_reinterpreted(self):
        target = pathlib.Path(self.temporary.name) / "other"
        target.write_text("#!/usr/bin/env python3\n", encoding="utf-8")
        target.chmod(0o755)
        completed = self.run_adapter(str(target))
        self.assertEqual(completed.returncode, 0, completed.stderr)
        observed = json.loads(completed.stdout)
        self.assertEqual(observed["argv"][1:], [str(target)])

    def test_unknown_loader_option_is_refused(self):
        completed = self.run_adapter(
            "/provider/lib64/ld-linux-x86-64.so.2",
            "--inhibit-cache",
            "/fixture/program",
        )
        self.assertEqual(completed.returncode, 64)
        self.assertIn("unsupported loader option", completed.stderr)

    def test_missing_target_is_refused(self):
        completed = self.run_adapter()
        self.assertEqual(completed.returncode, 64)
        self.assertIn("missing emulated executable", completed.stderr)


if __name__ == "__main__":
    unittest.main()

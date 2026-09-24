"""WD0 package custody on disposable user-owned roots."""
import argparse
import json
import pathlib
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import package


class PackageTests(unittest.TestCase):
    def fixture(self, root):
        source = root / "source"
        runtime = source / "bridge-manager/runtime"
        runtime.mkdir(parents=True)
        (runtime / "session.py").write_bytes(b"session fixture\n")
        (runtime / "ownership.py").write_bytes(b"ownership fixture\n")
        importer = source / "tools/wd0/import_installer.py"
        importer.parent.mkdir(parents=True)
        importer.write_bytes(b"importer fixture\n")
        manager = source / "linux-vst-bridge"
        manager.write_bytes(b"\x7fELF\x02\x01" + bytes(12) + b"\x3e\x00" + b"manager fixture\n")
        for args in [("init", "-q"), ("add", "."),
                     ("-c", "user.name=WD0", "-c", "user.email=wd0@example.invalid",
                      "commit", "-qm", "fixture")]:
            subprocess.run(["git", "-C", str(source), *args], check=True)
        output = root / "package"
        package.build(argparse.Namespace(source=source, manager=manager, output=output))
        return output

    def test_build_install_and_foreign_desktop_refusal(self):
        with tempfile.TemporaryDirectory() as temp:
            root = pathlib.Path(temp).resolve()
            pkg = self.fixture(root)
            meta = json.loads((pkg / "package.json").read_bytes())
            self.assertEqual(set(meta["files"]), set(package.NAMES))
            home = root / "home"
            managed = home / ".local/share/linux-vst-bridge/managed"
            managed.mkdir(parents=True, mode=0o700)
            with patch.object(pathlib.Path, "home", return_value=home):
                package.install(argparse.Namespace(package=pkg))
                package.install(argparse.Namespace(package=pkg))
                selection = managed / "daw-workspaces/tooling/selection.json"
                self.assertEqual(json.loads(selection.read_bytes())["generation"], meta["generation"])
                desktop = home / ".local/share/applications" / package.DESKTOP
                self.assertIn("workspace launch", desktop.read_text())
                desktop.write_text("foreign\n")
                with self.assertRaisesRegex(ValueError, "desktop entry foreign"):
                    package.install(argparse.Namespace(package=pkg))

    def test_source_mutation_and_artifact_change_refuse(self):
        with tempfile.TemporaryDirectory() as temp:
            root = pathlib.Path(temp).resolve()
            pkg = self.fixture(root)
            (pkg / "session.py").chmod(0o600)
            (pkg / "session.py").write_bytes(b"tampered")
            with self.assertRaisesRegex(ValueError, "artifact differs"):
                with patch.object(pathlib.Path, "home", return_value=root / "home"):
                    package.install(argparse.Namespace(package=pkg))
            (root / "source/bridge-manager/runtime/session.py").write_bytes(b"changed")
            with self.assertRaisesRegex(ValueError, "committed and clean"):
                package.build(argparse.Namespace(source=root / "source", manager=root / "source/linux-vst-bridge",
                                                 output=root / "another-package"))


if __name__ == "__main__":
    unittest.main()

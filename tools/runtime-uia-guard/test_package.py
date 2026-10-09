"""Corresponding source must be the inputs consumed by the built component."""
import io
import tarfile
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import build
import package


class CorrespondingSourceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.names = ["dlls/uiautomationcore/uia_client.c",
                      "dlls/uiautomationcore/tests/Makefile.in",
                      "dlls/uiautomationcore/tests/guard_provider.c"]
        self.source = self.root / "wine"
        for name in self.names:
            path = self.source / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(("consumed " + name + "\n").encode())
        (self.root / "build.sh").write_bytes(b"export XDG_CACHE_HOME=/work/cache\n")
        with tarfile.open(self.root / "wine.tar.gz", "w:gz") as archive:
            for name in self.names[:2] + ["COPYING.LIB", "unmodified.c"]:
                data = ("pristine " + name + "\n").encode()
                info = tarfile.TarInfo("wine-" + build.WINE + "/" + name)
                info.size = len(data)
                archive.addfile(info, io.BytesIO(data))
        self.receipt = {"source_inputs": {name: build.sha(self.source / name)
                                         for name in self.names},
                        "build_script_sha256": build.sha(self.root / "build.sh")}
        self.addCleanup(patch.stopall)
        patch.object(build, "ARCHIVE_SHA", build.sha(self.root / "wine.tar.gz")).start()

    def test_bundle_preserves_consumed_changes_and_pristine_remainder(self):
        output = self.root / "source.tar.gz"
        package.source_archive(output, self.root, self.receipt)
        with tarfile.open(output) as archive:
            for name in self.names:
                self.assertEqual(archive.extractfile("wine/" + name).read(),
                                 (self.source / name).read_bytes())
            self.assertEqual(archive.extractfile("wine/unmodified.c").read(),
                             b"pristine unmodified.c\n")
            self.assertEqual(archive.extractfile("build.sh").read(),
                             (self.root / "build.sh").read_bytes())

    def test_changed_consumed_source_is_rejected_before_archive_creation(self):
        for name in self.names:
            with self.subTest(name=name):
                path = self.source / name
                original = path.read_bytes()
                path.write_bytes(original + b"later edit\n")
                output = self.root / "not-created.tar.gz"
                with self.assertRaisesRegex(ValueError, "consumed source changed"):
                    package.source_archive(output, self.root, self.receipt)
                self.assertFalse(output.exists())
                path.write_bytes(original)

    def test_changed_consumed_recipe_is_rejected_before_archive_creation(self):
        (self.root / "build.sh").write_bytes(b"different configuration\n")
        output = self.root / "not-created.tar.gz"
        with self.assertRaisesRegex(ValueError, "consumed build recipe changed"):
            package.source_archive(output, self.root, self.receipt)
        self.assertFalse(output.exists())

    def test_changed_pristine_archive_is_rejected(self):
        archive = self.root / "wine.tar.gz"
        archive.write_bytes(archive.read_bytes() + b"changed")
        output = self.root / "not-created.tar.gz"
        with self.assertRaisesRegex(ValueError, "source archive changed"):
            package.source_archive(output, self.root, self.receipt)
        self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()

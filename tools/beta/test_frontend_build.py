"""Exact source and live-only qualification frontend build boundaries."""
import copy
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import frontend_build as frontend


def commit(root):
    subprocess.run(["git", "-C", str(root), "init", "-q"], check=True)
    subprocess.run(["git", "-C", str(root), "add", "."], check=True)
    subprocess.run(["git", "-C", str(root), "-c", "user.name=Fixture", "-c",
                    "user.email=fixture@example.invalid", "commit", "-qm", "Fixture"], check=True)


class FrontendBuild(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.bridge, self.ui = self.root / "bridge", self.root / "private"
        self.bridge.mkdir()
        self.ui.mkdir()
        for name in frontend.BRIDGE_CRATES:
            path = self.bridge / name / "Cargo.toml"
            path.parent.mkdir()
            path.write_text("fixture")
        commit(self.bridge)
        self.manifest = self.ui / "app/Cargo.toml"
        self.manifest.parent.mkdir()
        self.manifest.write_text("fixture manifest")
        self.manifest.with_name("Cargo.lock").write_text("fixture lock")
        commit(self.ui)
        public = frontend.source_identity(self.bridge)
        source = "git+" + frontend.BRIDGE_REPOSITORY + ".git?rev=" + public["head"] + "#" + public["head"]
        packages = [{"id": name, "name": name, "source": source,
                     "manifest_path": str(self.bridge / name / "Cargo.toml")}
                    for name in sorted(frontend.BRIDGE_CRATES)]
        packages.append({"id": "root", "name": "makepad-app", "source": None,
                         "manifest_path": str(self.manifest),
                         "targets": [{"name": frontend.BINARY, "kind": ["bin"]}]})
        nodes = [{"id": p["id"], "features": [], "deps": []} for p in packages[:-1]]
        nodes.append({"id": "root", "features": ["live"], "deps": [
            {"pkg": p["id"], "dep_kinds": [{"kind": None}]} for p in packages[:-1]]})
        self.metadata = {"workspace_root": str(self.manifest.parent), "packages": packages,
                         "resolve": {"root": "root", "nodes": nodes}}

    def describe(self, metadata=None):
        with patch.object(frontend, "release_metadata", return_value=metadata or self.metadata):
            return frontend.describe(self.bridge, self.ui, "app/Cargo.toml", "a" * 64, "internal_test")

    def test_two_distinct_source_identities_and_exact_lock_are_retained(self):
        record = self.describe()
        self.assertEqual(record["bridge_source"], frontend.source_identity(self.bridge))
        self.assertNotEqual(record["bridge_source"]["head"], record["frontend_source"]["head"])
        self.assertEqual(record["frontend_source"]["cargo_lock_sha256"],
                         frontend.sha(self.manifest.with_name("Cargo.lock")))
        self.assertNotIn(str(self.root), json.dumps(record))

    def test_different_public_revision_or_tree_refuses(self):
        metadata = copy.deepcopy(self.metadata)
        metadata["packages"][0]["source"] = metadata["packages"][0]["source"].split("#")[0] + "#" + "b" * 40
        with self.assertRaisesRegex(ValueError, "exact public Bridge revision"):
            self.describe(metadata)
        original = frontend.source_identity
        calls = 0

        def changed_tree(root):
            nonlocal calls
            calls += 1
            result = original(root)
            return {**result, "tree": "f" * 40} if calls > 2 else result

        with patch.object(frontend, "source_identity", side_effect=changed_tree):
            with self.assertRaisesRegex(ValueError, "resolved Bridge source tree differs"):
                self.describe()

    def test_dirty_private_source_and_sample_feature_refuse(self):
        self.manifest.write_text("changed manifest")
        with self.assertRaisesRegex(ValueError, "commit exact"):
            self.describe()
        self.manifest.write_text("fixture manifest")
        metadata = copy.deepcopy(self.metadata)
        next(n for n in metadata["resolve"]["nodes"] if n["id"] == "lvb-view-model")["features"] = ["samples"]
        with self.assertRaisesRegex(ValueError, "sample fixtures"):
            self.describe(metadata)

    def test_missing_client_and_foreign_path_dependency_refuse(self):
        metadata = copy.deepcopy(self.metadata)
        metadata["resolve"]["nodes"][-1]["deps"] = [
            d for d in metadata["resolve"]["nodes"][-1]["deps"] if d["pkg"] != "lvb-client"]
        with self.assertRaisesRegex(ValueError, "model, view model and client"):
            self.describe(metadata)
        metadata = copy.deepcopy(self.metadata)
        metadata["packages"][0]["source"] = None
        with self.assertRaisesRegex(ValueError, "path dependency escapes"):
            self.describe(metadata)

    def test_changed_binary_or_embedded_identity_refuses(self):
        metadata = self.describe()
        binary = self.root / frontend.BINARY
        binary.write_bytes(b"\x7fELFfixture")
        record = {**metadata, "artifact_sha256": frontend.sha(binary)}
        with patch.object(frontend, "describe", return_value=metadata):
            with patch.object(subprocess, "check_output", return_value=json.dumps(metadata).encode()):
                self.assertEqual(frontend.verify_record(record, self.bridge, self.ui, binary), record)
            with patch.object(subprocess, "check_output", return_value=b'{}'):
                with self.assertRaisesRegex(ValueError, "embedded build identity"):
                    frontend.verify_record(record, self.bridge, self.ui, binary)
            binary.write_bytes(b"\x7fELFchanged")
            with self.assertRaisesRegex(ValueError, "sources or artifact changed"):
                frontend.verify_record(record, self.bridge, self.ui, binary)


if __name__ == "__main__":
    unittest.main()

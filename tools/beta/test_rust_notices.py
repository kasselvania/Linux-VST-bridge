"""Qualification notices include exact-source vendored crates and omit dev edges."""
import hashlib
import copy
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import rust_notices
from test_frontend_build import commit


class FrontendNotices(unittest.TestCase):
    def test_exact_source_registry_and_root_licenses_are_captured(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bridge, private, output = root / "bridge", root / "private", root / "output"
            output.mkdir()
            for name in ("crates/cli", "native-vst3-proxy/backend"):
                path = bridge / name
                path.mkdir(parents=True)
                (path / "Cargo.toml").write_text("fixture")
                (path / "Cargo.lock").write_text("package = []")
            for name in ("app", "libs/vendor", "libs/development-only"):
                path = private / name
                path.mkdir(parents=True)
                (path / "Cargo.toml").write_text("fixture")
            (private / "LICENSE").write_text("Makepad exact root license")
            vendor_license = private / "libs/vendor/LICENSE"
            vendor_license.write_text("Vendored exact license")
            (private / "app/resources").mkdir()
            (private / "app/resources/OFL-BodyFont.txt").write_text("Bundled font license")
            (private / ".gitignore").write_text("NOTICE-unrecorded\napp/resources/OFL-unrecorded.txt\n")
            archive = root / "registry/cache/index/registry-fixture-1.0.crate"
            archive.parent.mkdir(parents=True)
            archive.write_bytes(b"locked archive fixture")
            checksum = hashlib.sha256(archive.read_bytes()).hexdigest()
            registry = root / "registry/src/index/registry-fixture-1.0"
            registry.mkdir(parents=True)
            (registry / "LICENSE").write_text("Registry exact license")
            (private / "app/Cargo.lock").write_text(
                '[[package]]\nname="registry-fixture"\nversion="1.0"\n'
                'source="registry+https://github.com/rust-lang/crates.io-index"\n'
                f'checksum="{checksum}"\n')
            packages = [
                dict(id="app", name="makepad-app", version="1.0", license="MIT", source=None,
                     manifest_path=str(private / "app/Cargo.toml")),
                dict(id="vendor", name="vendor", version="1.0", license="MIT", source=None,
                     manifest_path=str(private / "libs/vendor/Cargo.toml")),
                dict(id="dev", name="dev", version="1.0", license=None, source=None,
                     manifest_path=str(private / "libs/development-only/Cargo.toml")),
                dict(id="registry", name="registry-fixture", version="1.0", license="MIT",
                     source="registry+https://github.com/rust-lang/crates.io-index",
                     manifest_path=str(registry / "Cargo.toml")),
            ]
            nodes = [dict(id=p["id"], features=[], deps=[]) for p in packages]
            nodes[0]["deps"] = [{"pkg": name, "dep_kinds": [{"kind": kind}]}
                                for name, kind in (("vendor", None), ("registry", None), ("dev", "dev"))]
            metadata = {"packages": packages, "resolve": {"root": "app", "nodes": nodes}}
            public_metadata = {"packages": [dict(id="public", name="public", source=None,
                manifest_path=str(bridge / "crates/cli/Cargo.toml"))],
                "resolve": {"root": "public", "nodes": [dict(id="public", deps=[])]}}
            commit(bridge)
            commit(private)
            identity = rust_notices.qualification.source_identity(private)
            record = {"bridge_source": rust_notices.qualification.source_identity(bridge),
                      "frontend_source": {**identity, "manifest": "app/Cargo.toml"}}
            original_check_output = rust_notices.subprocess.check_output

            def cargo_metadata(manifest, **_):
                materialized = Path(manifest).parent.parent
                result = copy.deepcopy(metadata)
                for package in result["packages"]:
                    path = Path(package["manifest_path"])
                    if path.is_relative_to(private):
                        package["manifest_path"] = str(materialized / path.relative_to(private))
                return result

            def check_output(command, **kwargs):
                if command[0] != "cargo":
                    return original_check_output(command, **kwargs)
                result = copy.deepcopy(public_metadata)
                result["packages"][0]["manifest_path"] = command[command.index("--manifest-path") + 1]
                return json.dumps(result).encode()

            def capture():
                with patch.dict(os.environ, {"CARGO_HOME": str(root / "empty-cache")}), \
                        patch.object(rust_notices.qualification, "release_metadata", side_effect=cargo_metadata), \
                        patch.object(rust_notices.subprocess, "check_output", side_effect=check_output):
                    return rust_notices.capture(bridge, output, frontend_source=private, frontend_build=record)

            inventory = capture()
            self.assertEqual({p["name"] for p in inventory}, {"makepad-app", "vendor", "registry-fixture"})
            vendor = next(p for p in inventory if p["name"] == "vendor")
            self.assertEqual(vendor["source"], {"kind": "git", **identity, "path": "libs/vendor"})
            notices = (output / "Rust-Dependency-Notices.txt").read_text()
            for text in ("Makepad exact root license", "Vendored exact license", "Registry exact license",
                         "Bundled font license"):
                self.assertIn(text, notices)
            self.assertNotIn(str(root), (output / "Rust-Dependency-Inventory.json").read_text())
            # Both ignored root notices and ignored bundled-font notices used
            # to enter the inventory under this unchanged Git head/tree.
            for text in ("unrecorded notice A", "unrecorded notice B"):
                (private / "NOTICE-unrecorded").write_text(text)
                (private / "app/resources/OFL-unrecorded.txt").write_text(text)
                self.assertEqual(rust_notices.qualification.source_identity(private), identity)
                self.assertEqual(capture(), inventory)
                self.assertNotIn(text, (output / "Rust-Dependency-Notices.txt").read_text())
            vendor_license.unlink()
            commit(private)
            record["frontend_source"].update(rust_notices.qualification.source_identity(private))
            with self.assertRaisesRegex(ValueError, "Missing exact-source license"):
                capture()
            vendor_license.write_text("Vendored exact license")
            commit(private)
            record["frontend_source"].update(rust_notices.qualification.source_identity(private))
            archive.write_bytes(b"changed archive")
            with self.assertRaisesRegex(ValueError, "Registry archive differs"):
                capture()


if __name__ == "__main__":
    unittest.main()

"""Bridge application ownership and exact private toolkit build boundaries."""
import copy
from contextlib import contextmanager
import hashlib
import json
import os
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
        self.private_manifest = self.ui / "Cargo.toml"
        self.private_manifest.write_text("fixture toolkit manifest")
        self.private_manifest.with_name("Cargo.lock").write_text("fixture lock")
        for name in frontend.TOOLKIT_CRATES:
            path = self.ui / name / "Cargo.toml"
            path.parent.mkdir()
            path.write_text("fixture")
        commit(self.ui)
        self.manifest = self.bridge / frontend.WINDOW_MANIFEST
        self.manifest.parent.mkdir(parents=True)
        self.manifest.with_name("Cargo.lock").write_text("fixture window lock")
        self.pin_toolkit()
        private = frontend.source_identity(self.ui)
        source = "git+" + frontend.TOOLKIT_REPOSITORY + ".git?rev=" + private["head"] + "#" + private["head"]
        packages = [{"id": name, "name": name, "source": None,
                     "manifest_path": str(self.bridge / name / "Cargo.toml")}
                    for name in sorted(frontend.BRIDGE_CRATES)]
        packages += [{"id": name, "name": name, "source": source,
                      "manifest_path": str(self.ui / name / "Cargo.toml")}
                     for name in sorted(frontend.TOOLKIT_CRATES)]
        packages.append({"id": "root", "name": "lvb-window", "source": None,
                         "manifest_path": str(self.manifest),
                         "targets": [{"name": name, "kind": ["bin"]} for name in frontend.BINARIES]})
        nodes = [{"id": p["id"], "features": [], "deps": []} for p in packages[:-1]]
        nodes.append({"id": "root", "features": ["live"], "deps": [
            {"pkg": p["id"], "dep_kinds": [{"kind": None}]} for p in packages[:-1]]})
        self.metadata = {"workspace_root": str(self.manifest.parent), "packages": packages,
                         "resolve": {"root": "root", "nodes": nodes}}

    def pin_toolkit(self):
        head = frontend.source_identity(self.ui)["head"]
        self.manifest.write_text('[dependencies]\n' + '\n'.join(
            f'{name} = {{ git = "{frontend.TOOLKIT_REPOSITORY}.git", rev = "{head}" }}'
            for name in sorted(frontend.TOOLKIT_CRATES)))
        commit(self.bridge)

    def describe(self, metadata=None):
        with patch.object(frontend, "release_metadata", return_value=metadata or self.metadata) as cargo:
            record = frontend.describe_sources(self.bridge, self.ui, frontend.WINDOW_MANIFEST.as_posix(), "a" * 64, "internal_test")
            self.assertEqual(cargo.call_args.args[0], self.manifest.resolve())
            return record

    def test_application_identity_and_lock_keep_existing_package_contract(self):
        record = self.describe()
        self.assertEqual(record["bridge_source"], frontend.source_identity(self.bridge))
        self.assertEqual(record["bridge_source"]["head"], record["frontend_source"]["head"])
        self.assertEqual(record["frontend_source"]["cargo_lock_sha256"],
                         frontend.sha(self.manifest.with_name("Cargo.lock")))
        self.assertNotIn(str(self.root), json.dumps(record))
        # The existing package validator accepts this unchanged schema.
        import sys
        sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "pkg0"))
        from assemble import validate_frontend_build
        public = record["bridge_source"]
        validate_frontend_build({**record, "artifact_sha256": "c" * 64},
                                public["head"], public["tree"], "c" * 64)

    def test_old_private_application_route_refuses(self):
        with self.assertRaisesRegex(ValueError, "application is crates/window"):
            frontend.describe_sources(self.bridge, self.ui, "apps/bridge-pilot/Cargo.toml",
                                      "a" * 64, "internal_test")

    def test_different_private_revision_or_tree_refuses(self):
        metadata = copy.deepcopy(self.metadata)
        package = next(p for p in metadata["packages"] if p["name"] == "makepad-widgets")
        package["source"] = package["source"].split("#")[0] + "#" + "b" * 40
        with self.assertRaisesRegex(ValueError, "exact private toolkit revision"):
            self.describe(metadata)
        original = frontend.git_identity
        calls = 0

        def changed_tree(root):
            nonlocal calls
            calls += 1
            result = original(root)
            return {**result, "tree": "f" * 40} if calls > 2 else result

        with patch.object(frontend, "git_identity", side_effect=changed_tree):
            with self.assertRaisesRegex(ValueError, "resolved toolkit source tree differs"):
                self.describe()

    def test_dirty_source_and_sample_features_refuse(self):
        original = self.private_manifest.read_text()
        self.private_manifest.write_text("changed manifest")
        with self.assertRaisesRegex(ValueError, "commit exact"):
            self.describe()
        self.private_manifest.write_text(original)
        for name in ("root", "lvb-view-model"):
            metadata = copy.deepcopy(self.metadata)
            next(n for n in metadata["resolve"]["nodes"] if n["id"] == name)["features"].append("samples")
            with self.assertRaisesRegex(ValueError, "only the live feature|sample fixtures"):
                self.describe(metadata)

    def test_missing_client_or_toolkit_and_foreign_path_dependency_refuse(self):
        for name, error in (("lvb-client", "model, view model and client"),
                            ("makepad-widgets", "both exact private toolkit crates")):
            metadata = copy.deepcopy(self.metadata)
            metadata["resolve"]["nodes"][-1]["deps"] = [
                d for d in metadata["resolve"]["nodes"][-1]["deps"] if d["pkg"] != name]
            with self.assertRaisesRegex(ValueError, error):
                self.describe(metadata)
        metadata = copy.deepcopy(self.metadata)
        package = next(p for p in metadata["packages"] if p["name"] == "makepad-widgets")
        package["source"] = None
        with self.assertRaisesRegex(ValueError, "path dependency escapes"):
            self.describe(metadata)

    def test_manifest_pin_must_match_recorded_toolkit(self):
        self.manifest.write_text(self.manifest.read_text().replace(frontend.source_identity(self.ui)["head"], "b" * 40))
        commit(self.bridge)
        with self.assertRaisesRegex(ValueError, "pin both private toolkit crates"):
            self.describe()

    def test_both_native_executable_targets_are_required(self):
        for missing in frontend.BINARIES:
            metadata = copy.deepcopy(self.metadata)
            metadata['packages'][-1]['targets'] = [t for t in metadata['packages'][-1]['targets']
                                                   if t['name'] != missing]
            with self.assertRaisesRegex(ValueError, 'both native application executables'):
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

    def test_native_pair_refuses_missing_changed_or_different_installer_without_executing_it(self):
        metadata = self.describe()
        installer_metadata = {**metadata, 'binary': frontend.INSTALLER}
        binaries = self.root / 'binaries'
        binaries.mkdir()
        manager, installer = binaries / frontend.BINARY, binaries / frontend.INSTALLER
        manager.write_bytes(b'\x7fELFmanager')
        installer.write_bytes(b'\x7fELF' + json.dumps(installer_metadata, sort_keys=True, separators=(',', ':')).encode())
        records = [{**info, 'artifact_sha256': frontend.sha(binary)}
                   for info, binary in ((metadata, manager), (installer_metadata, installer))]
        record_path = binaries / 'frontend-build.json'
        record_path.write_text(json.dumps(records[0]))
        installer_record = binaries / 'installer-build.json'
        with patch.object(frontend, 'describe', side_effect=lambda *args: {**metadata, 'binary': args[-1]}), \
                patch.object(subprocess, 'check_output', return_value=json.dumps(metadata).encode()) as executed:
            with self.assertRaises(FileNotFoundError):
                frontend.verify_pair(record_path, self.bridge, self.ui)
            installer_record.write_text(json.dumps(records[1]))
            self.assertEqual(frontend.verify_pair(record_path, self.bridge, self.ui), tuple(records))
            self.assertTrue(all(call.args[0] == [str(manager.resolve()), '--package-info']
                                for call in executed.call_args_list))
            installer.write_bytes(b'\x7fELFold egui installer')
            with self.assertRaisesRegex(ValueError, 'artifact changed'):
                frontend.verify_pair(record_path, self.bridge, self.ui)
            installer_record.write_text(json.dumps({**records[1], 'artifact_sha256': frontend.sha(installer)}))
            with self.assertRaisesRegex(ValueError, 'installer embedded build identity'):
                frontend.verify_pair(record_path, self.bridge, self.ui)
            installer_record.write_text(json.dumps({**records[1], 'portable_public_key': 'b' * 64}))
            with self.assertRaisesRegex(ValueError, 'sources or artifact changed'):
                frontend.verify_pair(record_path, self.bridge, self.ui)
        other_key = {**installer_metadata, 'portable_public_key': 'b' * 64}
        installer.write_bytes(b'\x7fELF' + json.dumps(other_key, sort_keys=True, separators=(',', ':')).encode())
        installer_record.write_text(json.dumps({**other_key, 'artifact_sha256': frontend.sha(installer)}))
        with patch.object(frontend, 'describe', side_effect=lambda *args:
                          {**metadata, 'binary': args[-1], 'portable_public_key': args[3]}), \
                patch.object(subprocess, 'check_output', return_value=json.dumps(metadata).encode()):
            with self.assertRaisesRegex(ValueError, 'manager and installer build identities differ'):
                frontend.verify_pair(record_path, self.bridge, self.ui)

    def test_one_build_produces_both_binaries_and_separate_exact_records(self):
        metadata = self.describe()
        output, target = self.root / 'output', self.root / 'target'
        original_run, original_output = subprocess.run, subprocess.check_output
        builds = []

        @contextmanager
        def sources(*_):
            yield self.bridge, self.ui, {}

        def run(command, **kwargs):
            if command[0] != 'cargo':
                return original_run(command, **kwargs)
            builds.append(command)
            self.assertEqual([command[i + 1] for i, part in enumerate(command) if part == '--bin'],
                             list(frontend.BINARIES))
            self.assertEqual(kwargs['env']['CARGO_BUILD_JOBS'], '2')
            for name, variable in ((frontend.BINARY, 'LVB_FRONTEND_PACKAGE_INFO'),
                                   (frontend.INSTALLER, 'LVB_INSTALLER_PACKAGE_INFO')):
                declaration = kwargs['env'][variable]
                self.assertEqual(json.loads(declaration), {**metadata, 'binary': name})
                path = target / frontend.TARGET / 'release' / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b'\x7fELF' + declaration.encode())
            return subprocess.CompletedProcess(command, 0)

        def check_output(command, **kwargs):
            if command[-1] == '--package-info':
                self.assertEqual(Path(command[0]).name, frontend.BINARY)
                return json.dumps(metadata).encode()
            return original_output(command, **kwargs)

        with patch.object(frontend, 'recorded_sources', sources), \
                patch.object(frontend, 'describe_sources', return_value=metadata), \
                patch.object(frontend, 'describe', side_effect=lambda *args: {**metadata, 'binary': args[-1]}), \
                patch.object(subprocess, 'run', side_effect=run), \
                patch.object(subprocess, 'check_output', side_effect=check_output):
            record = frontend.build(self.bridge, self.ui, frontend.WINDOW_MANIFEST, output, target,
                                    metadata['portable_public_key'], 'internal_test')
            pair = frontend.verify_pair(output / 'frontend-build.json', self.bridge, self.ui)
        self.assertEqual(len(builds), 1)
        self.assertEqual(pair[0], record)
        self.assertEqual(pair[1]['binary'], frontend.INSTALLER)
        for name in frontend.BINARIES:
            self.assertEqual((output / name).stat().st_mode & 0o777, 0o555)

    def test_ignored_application_dependency_cannot_share_recorded_identity(self):
        (self.bridge / ".gitignore").write_text("ignored/\n")
        commit(self.bridge)
        dependency = self.bridge / "ignored/Cargo.toml"
        dependency.parent.mkdir()
        metadata = copy.deepcopy(self.metadata)
        metadata["packages"].append(dict(id="ignored", name="ignored", source=None,
                                         manifest_path=str(dependency)))
        metadata["resolve"]["nodes"][-1]["deps"].append(
            {"pkg": "ignored", "dep_kinds": [{"kind": None}]})
        metadata["resolve"]["nodes"].append(dict(id="ignored", features=[], deps=[]))
        identity = frontend.source_identity(self.bridge)
        for contents in ("first unrecorded dependency", "different unrecorded dependency"):
            dependency.write_text(contents)
            self.assertEqual(frontend.source_identity(self.bridge), identity)
            with self.assertRaisesRegex(ValueError, "not a tracked regular file"):
                self.describe(metadata)

    def test_fresh_materializations_exclude_ignored_sources_and_cargo_sentinels(self):
        (self.ui / ".gitignore").write_text("ignored.rs\n")
        commit(self.ui)
        (self.ui / "ignored.rs").write_text("unrecorded source")
        (self.bridge / ".git/info/exclude").write_text(".cargo-ok\n")
        (self.bridge / ".cargo-ok").write_text("Cargo sentinel")
        with patch.dict(os.environ, {"CARGO_HOME": str(self.root / "empty-cache")}):
            with frontend.recorded_sources(self.bridge, self.ui) as (public, private, env):
                self.assertNotEqual(private, self.ui)
                self.assertFalse((private / "ignored.rs").exists())
                self.assertFalse((public / ".cargo-ok").exists())
                self.assertEqual(frontend.source_identity(private), frontend.source_identity(self.ui))
                self.assertEqual(frontend.source_identity(public), frontend.source_identity(self.bridge))
                self.assertFalse((Path(env["CARGO_HOME"]) / "git/checkouts").exists())
                self.assertFalse((Path(env["CARGO_HOME"]) / "registry/src").exists())
            self.assertFalse(private.exists())

    def test_cargo_sentinel_does_not_relax_tracked_source_identity(self):
        checkout = self.root / "cargo/git/checkouts/makepad/pinned"
        subprocess.run(["git", "clone", "-q", str(self.ui), str(checkout)], check=True)
        (checkout / ".cargo-ok").write_text("Cargo extraction complete")
        metadata = copy.deepcopy(self.metadata)
        for package in metadata["packages"]:
            if package["name"] in frontend.TOOLKIT_CRATES:
                package["manifest_path"] = str(checkout / package["name"] / "Cargo.toml")
        with patch.object(frontend, "release_metadata", return_value=metadata):
            args = (self.bridge, self.ui, frontend.WINDOW_MANIFEST.as_posix(), "a" * 64, "internal_test")
            env = {"CARGO_HOME": str(self.root / "cargo")}
            frontend.describe_sources(*args, env)
            (checkout / "makepad-widgets/Cargo.toml").write_text("unrecorded tracked edit")
            with self.assertRaisesRegex(ValueError, "resolved toolkit source tree differs"):
                frontend.describe_sources(*args, env)

    def test_only_locked_registry_archive_bytes_enter_the_fresh_cache(self):
        cache = self.root / "cache"
        archive = cache / "registry/cache/index/fixture-1.0.crate"
        archive.parent.mkdir(parents=True)
        data = b"exact locked archive"
        archive.write_bytes(data)
        self.manifest.with_name("Cargo.lock").write_text(
            '[[package]]\nname="fixture"\nversion="1.0"\nsource="registry+https://example.invalid"\n'
            f'checksum="{hashlib.sha256(data).hexdigest()}"\n')
        commit(self.bridge)
        with patch.dict(os.environ, {"CARGO_HOME": str(cache)}):
            with frontend.recorded_sources(self.bridge, self.ui) as (_, _, env):
                copied = Path(env["CARGO_HOME"]) / archive.relative_to(cache)
                self.assertEqual(copied.read_bytes(), data)
                self.assertFalse(copied.is_symlink())
            archive.write_bytes(b"changed cached archive")
            with frontend.recorded_sources(self.bridge, self.ui) as (_, _, env):
                self.assertFalse((Path(env["CARGO_HOME"]) / archive.relative_to(cache)).exists())
            self.assertEqual(archive.read_bytes(), b"changed cached archive")


if __name__ == "__main__":
    unittest.main()

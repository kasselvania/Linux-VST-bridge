"""Debian package-format tests over PKG0's actual declared payload shape."""

import hashlib
import copy
import gzip
import io
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "pkg0"))
import assemble  # noqa: E402
import test_assemble  # noqa: E402
import deb  # noqa: E402
import release  # noqa: E402


class DebianPackage(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.files = []
        inputs = self.root / "inputs"
        inputs.mkdir()
        for index, (name, kind) in enumerate(assemble.REQUIRED.items()):
            prefix = b"\x7fELF" if kind in ("manager", "frontend") else b"MZ" if kind == "windows_host" else b""
            if name in deb.SUPERVISOR_PATHS:
                prefix = bytes.fromhex("f30d0d0a") + (0).to_bytes(4, "little") + b"fixture0"
            self.add(inputs, name, kind, prefix + f"fixture-{index}".encode())
        self.add(inputs, "usr/lib/linux-vst-bridge/proxy/Test.so", "proxy", b"\x7fELFproxy")
        self.add(inputs, "usr/lib/linux-vst-bridge/self-test/Test.vst3", "fixture", b"MZfixture")
        self.add(inputs, "usr/lib/linux-vst-bridge/profiles/test.json", "profile", b"{}")
        self.add(inputs, "usr/share/doc/linux-vst-bridge-beta/licenses/Bridge.txt", "license", b"license")
        spec = {"schema": 1, "version": "0.1.0beta1", "source_head": "a" * 40,
                "source_tree": "b" * 40, "operator_schema": 14,
                "external_runtime": {"id": "exact-proton-slr", "manifest_sha256": "c" * 64},
                "files": self.files}
        self.staged = self.root / "staged"
        assemble.build(spec, self.staged, 1_234_567_890)
        self.package = self.root / "linux-vst-bridge-beta_0.1.0beta1-1_amd64.deb"

    def add(self, inputs, name, kind, data):
        source = inputs / str(len(self.files))
        source.write_bytes(data)
        row = {"destination": name, "source": str(source),
               "sha256": hashlib.sha256(data).hexdigest(), "kind": kind,
               "component": "test", "mode": "0555" if kind in ("manager", "frontend", "proxy") else "0444"}
        if kind in ("manager", "frontend"):
            row["build_head"] = "a" * 40
            row["build_tree"] = "b" * 40
        self.files.append(row)

    def build(self):
        return deb.build(self.staged, self.package, 1_234_567_890)

    def stage_with_python_magics(self, session_magic, ownership_magic, *, flags=0):
        for row in self.files:
            magic = (session_magic if row["destination"] == deb.SUPERVISOR_PATHS[0]
                     else ownership_magic if row["destination"] == deb.SUPERVISOR_PATHS[1]
                     else None)
            if magic is not None:
                data = magic + flags.to_bytes(4, "little") + b"fixture0body"
                Path(row["source"]).write_bytes(data)
                row["sha256"] = hashlib.sha256(data).hexdigest()
        spec = {"schema": 1, "version": "0.1.0beta1", "source_head": "a" * 40,
                "source_tree": "b" * 40, "operator_schema": 14,
                "external_runtime": {"id": "exact-proton-slr", "manifest_sha256": "c" * 64},
                "files": self.files}
        stage = self.root / "alternate-staged"
        assemble.build(spec, stage, 1_234_567_890)
        return stage

    def test_exact_deb_and_reproducibility(self):
        identity = self.build()
        self.assertEqual(identity["source_head"], "a" * 40)
        self.assertEqual(identity["architecture"], "amd64")
        self.assertEqual(deb.verify(self.package, self.staged)["package_sha256"],
                         identity["package_sha256"])
        with tempfile.TemporaryDirectory() as unpack:
            contents = deb.ar_entries(self.package, unpack)
            self.assertEqual(list(contents), ["debian-binary", "control.tar.gz", "data.tar.gz"])
            with tarfile.open(contents["data.tar.gz"], mode="r:gz") as archive:
                members = archive.getmembers()
                adoption = json.load(archive.extractfile(assemble.ADOPTION_MANIFEST))
                self.assertEqual(adoption["external_runtime"]["id"], "exact-proton-slr")
                self.assertFalse(any("proton" in member.name.lower() for member in members))
                directories = {member.name.rstrip("/") for member in members if member.isdir()}
                self.assertEqual(directories,
                                 set(deb.parent_directories(row["destination"]
                                                            for row in json.loads((self.staged / "RELEASE_MANIFEST.json").read_bytes())["files"])))
        clone = self.root / "repeat" / self.package.name
        deb.build(self.staged, clone, 1_234_567_890)
        self.assertEqual(self.package.read_bytes(), clone.read_bytes())

    def test_pkg1_kit_remains_exact_adoption_authority(self):
        fixture = test_assemble.PackageAssembly("test_package_kit_is_bound_to_adoption_and_host_pair")
        fixture.setUp()
        try:
            for row in fixture.files:
                if row["destination"] in deb.SUPERVISOR_PATHS:
                    data = bytes.fromhex("f30d0d0a") + (0).to_bytes(4, "little") + b"fixture0body"
                    Path(row["source"]).write_bytes(data)
                    row["sha256"] = hashlib.sha256(data).hexdigest()
            fixture.add_kit()
            staged = fixture.root / "portable-kit"
            assemble.build(fixture.spec, staged, 1_234_567_890)
            package = fixture.root / self.package.name
            identity = deb.build(staged, package, 1_234_567_890)
            self.assertEqual(identity["package_sha256"], deb.verify(package, staged)["package_sha256"])
            with tempfile.TemporaryDirectory() as unpack:
                entries = deb.ar_entries(package, unpack)
                with tarfile.open(entries["data.tar.gz"], mode="r:gz") as archive:
                    adoption = json.load(archive.extractfile(assemble.ADOPTION_MANIFEST))
                    self.assertEqual(adoption["schema"], 2)
                    self.assertIn(assemble.KIT_DESTINATION, archive.getnames())
            release_manifest = staged / "RELEASE_MANIFEST.json"
            original = release_manifest.read_bytes()
            forged = json.loads(original)
            next(row for row in forged["files"] if row["kind"] == "preparation_kit")["destination"] = (
                "usr/lib/linux-vst-bridge/preparation/other-kit.zip")
            release_manifest.write_bytes(assemble.canonical(forged))
            with self.assertRaisesRegex(ValueError, "preparation kit destination"):
                deb.verify(package, staged)
            release_manifest.write_bytes(original)
        finally:
            fixture.doCleanups()

    def test_system_package_manager_parses_exact_metadata_when_available(self):
        self.build()
        if not shutil.which("dpkg-deb"):
            self.skipTest("dpkg-deb is unavailable on this host")
        detail = subprocess.check_output(["dpkg-deb", "-f", str(self.package)], text=True)
        self.assertIn("Package: linux-vst-bridge-beta\n", detail)
        self.assertIn("Architecture: amd64\n", detail)
        self.assertIn("Version: 0.1.0beta1-1\n", detail)
        self.assertIn("Depends: libc6 (>= 2.39)", detail)
        self.assertIn("libxkbcommon-x11-0", detail)
        self.assertIn("libxcursor1", detail)
        self.assertIn("libxi6", detail)
        self.assertIn("python3 (>= 3.13), python3 (<< 3.14)", detail)
        data = subprocess.check_output(["dpkg-deb", "-c", str(self.package)], text=True)
        self.assertIn("usr/share/linux-vst-bridge/pkg0-manifest.json", data)
        self.assertIn("usr/share/applications/linux-vst-bridge-setup.desktop", data)

    def test_python_314_bytecode_declares_exact_runtime_range(self):
        stage = self.stage_with_python_magics(bytes.fromhex("2b0e0d0a"),
                                              bytes.fromhex("2b0e0d0a"))
        package = self.root / "python314" / self.package.name
        deb.build(stage, package, 1_234_567_890)
        self.assertEqual(deb.verify(package, stage)["files"], len(self.files) + 2)
        with tempfile.TemporaryDirectory() as unpack:
            entries = deb.ar_entries(package, unpack)
            with tarfile.open(entries["control.tar.gz"], mode="r:gz") as archive:
                control = archive.extractfile("./control").read()
        self.assertIn(b"python3 (>= 3.14), python3 (<< 3.15)", control)
        self.assertNotIn(b"python3 (>= 3.13)", control)

    def test_mixed_or_unknown_python_bytecode_refuses_before_package_creation(self):
        for label, session, ownership, flags in (
            ("mixed", bytes.fromhex("f30d0d0a"), bytes.fromhex("2b0e0d0a"), 0),
            ("unknown", b"\xff\xff\xff\xff", b"\xff\xff\xff\xff", 0),
            ("malformed_header", bytes.fromhex("f30d0d0a"), bytes.fromhex("f30d0d0a"), 2),
        ):
            with self.subTest(label=label):
                output = self.root / label / self.package.name
                with self.assertRaisesRegex(ValueError, "supervisor Python"):
                    self.stage_with_python_magics(session, ownership, flags=flags)
                with self.assertRaisesRegex(ValueError, "supervisor Python"):
                    deb.python_abi({name: magic + flags.to_bytes(4, "little") + bytes(8)
                                    for name, magic in zip(deb.SUPERVISOR_PATHS,
                                                           (session, ownership))})
                self.assertFalse(output.exists())
                self.assertFalse((self.root / "alternate-staged").exists())

    def test_changed_payload_and_manifest_refuse(self):
        payload = self.staged / "payload.tar"
        payload.write_bytes(payload.read_bytes() + b"changed")
        with self.assertRaisesRegex(ValueError, "payload digest differs"):
            self.build()
        self.assertFalse(self.package.exists())

    def test_forged_release_roles_refuse_build_verify_and_sign(self):
        self.build()
        original = json.loads((self.staged / "RELEASE_MANIFEST.json").read_bytes())
        forged = [
            ("extra command", lambda rows: rows.append({
                "destination": "usr/bin/foreign-command", "sha256": "d" * 64,
                "size": 7, "mode": "0444", "kind": "guide", "component": "fixture"})),
            ("required role", lambda rows: next(row for row in rows if row["kind"] == "manager").update(
                kind="guide", mode="0444")),
            ("generated component", lambda rows: next(row for row in rows if row["kind"] == "system_desktop").update(
                component="fixture")),
            ("executable mode", lambda rows: next(row for row in rows if row["kind"] == "proxy").update(
                mode="0444")),
        ]
        gpg_home = self.root / "gpg-home"
        gpg_home.mkdir()
        for label, mutate in forged:
            with self.subTest(label=label):
                manifest = copy.deepcopy(original)
                mutate(manifest["files"])
                (self.staged / "RELEASE_MANIFEST.json").write_bytes(assemble.canonical(manifest))
                with self.assertRaises(ValueError):
                    deb.build(self.staged, self.root / "new" / self.package.name, 1_234_567_890)
                with self.assertRaises(ValueError):
                    deb.verify(self.package, self.staged)
                with self.assertRaises(ValueError):
                    release.sign(self.package, self.staged, gpg_home, "0" * 40,
                                 "internal_test", self.root / "forged-signed")
        (self.staged / "RELEASE_MANIFEST.json").write_bytes(assemble.canonical(original))
        self.assertEqual(deb.verify(self.package, self.staged)["files"], len(self.files) + 2)

    def test_extra_command_refuses_with_self_consistent_payload_and_digest(self):
        self.build()
        payload = self.staged / "payload.tar"
        manifest_path = self.staged / "RELEASE_MANIFEST.json"
        forged = json.loads(manifest_path.read_bytes())
        body = b"foreign"
        archive_bytes = io.BytesIO()
        with tarfile.open(payload, mode="r:") as source, tarfile.open(
                fileobj=archive_bytes, mode="w", format=tarfile.PAX_FORMAT) as target:
            for member in source:
                target.addfile(member, source.extractfile(member))
            member = tarfile.TarInfo("usr/bin/foreign-command")
            member.size = len(body)
            member.mode = 0o444
            member.uid = member.gid = 0
            member.mtime = 1_234_567_890
            target.addfile(member, io.BytesIO(body))
        forged["files"].append({
            "destination": member.name, "sha256": hashlib.sha256(body).hexdigest(),
            "size": len(body), "mode": "0444", "kind": "guide", "component": "fixture"})
        forged["payload_sha256"] = hashlib.sha256(archive_bytes.getvalue()).hexdigest()
        payload.write_bytes(archive_bytes.getvalue())
        manifest_path.write_bytes(assemble.canonical(forged))
        with self.assertRaisesRegex(ValueError, "file role and destination disagree"):
            deb.build(self.staged, self.root / "forged" / self.package.name, 1_234_567_890)
        with self.assertRaisesRegex(ValueError, "file role and destination disagree"):
            deb.verify(self.package, self.staged)
        gpg_home = self.root / "gpg-home"
        gpg_home.mkdir()
        with self.assertRaisesRegex(ValueError, "file role and destination disagree"):
            release.sign(self.package, self.staged, gpg_home, "0" * 40,
                         "internal_test", self.root / "forged-signed")

    def test_missing_and_symlinked_input_refuse_without_output(self):
        payload = self.staged / "payload.tar"
        original = self.root / "original.tar"
        payload.rename(original)
        payload.symlink_to(original)
        with self.assertRaisesRegex(ValueError, "absolute regular file"):
            self.build()
        self.assertFalse(self.package.exists())

    def test_changed_archive_and_extra_metadata_refuse(self):
        self.build()
        original = self.package.read_bytes()
        self.package.write_bytes(original + b"foreign")
        with self.assertRaises(ValueError):
            deb.verify(self.package, self.staged)
        self.package.write_bytes(original)
        self.assertEqual(deb.verify(self.package, self.staged)["files"], len(self.files) + 2)

    def test_control_dependency_and_script_injection_refuse(self):
        self.build()
        for name, control, extra in (
            ("dependency", deb.control_bytes({"version": "0.1.0beta1", "pkgrel": 1},
                 deb.PYTHON_ABIS[bytes.fromhex("f30d0d0a")]).replace(
                b"libc6 (>= 2.39)", b"libc6"), None),
            ("script", deb.control_bytes({"version": "0.1.0beta1", "pkgrel": 1},
                 deb.PYTHON_ABIS[bytes.fromhex("f30d0d0a")]), b"exit 0\n"),
            ("python", deb.control_bytes({"version": "0.1.0beta1", "pkgrel": 1},
                 deb.PYTHON_ABIS[bytes.fromhex("f30d0d0a")]).replace(
                     b"python3 (>= 3.13), python3 (<< 3.14)", b"python3"), None),
        ):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as unpack:
                entries = deb.ar_entries(self.package, unpack)
                control_gz = io.BytesIO()
                with gzip.GzipFile(fileobj=control_gz, mode="wb", mtime=1_234_567_890, filename="") as compressed:
                    with tarfile.open(fileobj=compressed, mode="w|", format=tarfile.GNU_FORMAT) as archive:
                        for filename, body in [("./control", control)] + ([('./postinst', extra)] if extra else []):
                            member = tarfile.TarInfo(filename)
                            member.size = len(body)
                            member.mode = 0o755 if extra and filename == "./postinst" else 0o644
                            archive.addfile(member, io.BytesIO(body))
                altered = self.root / name / self.package.name
                altered.parent.mkdir()
                with altered.open("wb") as package:
                    package.write(b"!<arch>\n")
                    for member_name, body in [
                        ("debian-binary", b"2.0\n"),
                        ("control.tar.gz", control_gz.getvalue()),
                        ("data.tar.gz", entries["data.tar.gz"].read_bytes()),
                    ]:
                        deb.ar_member(package, member_name, io.BytesIO(body), len(body), 1_234_567_890)
                with self.assertRaisesRegex(ValueError, "Debian control"):
                    deb.verify(altered, self.staged)

    def test_missing_parent_directory_refuses(self):
        self.build()
        with tempfile.TemporaryDirectory() as unpack:
            entries = deb.ar_entries(self.package, unpack)
            data_gz = io.BytesIO()
            with gzip.GzipFile(fileobj=data_gz, mode="wb", mtime=1_234_567_890,
                               filename="") as compressed:
                with tarfile.open(fileobj=compressed, mode="w|", format=tarfile.PAX_FORMAT) as target:
                    with tarfile.open(entries["data.tar.gz"], mode="r:gz") as source:
                        for member in source:
                            if member.isdir() and member.name.rstrip("/") == "usr/lib":
                                continue
                            target.addfile(member, source.extractfile(member) if member.isfile() else None)
            altered = self.root / "missing-directory" / self.package.name
            altered.parent.mkdir()
            with altered.open("wb") as package:
                package.write(b"!<arch>\n")
                for member_name, body in (
                    ("debian-binary", b"2.0\n"),
                    ("control.tar.gz", entries["control.tar.gz"].read_bytes()),
                    ("data.tar.gz", data_gz.getvalue()),
                ):
                    deb.ar_member(package, member_name, io.BytesIO(body), len(body), 1_234_567_890)
            with self.assertRaisesRegex(ValueError, "payload incomplete"):
                deb.verify(altered, self.staged)

    def test_output_name_and_duplicate_build_refuse(self):
        wrong = self.root / "wrong.deb"
        with self.assertRaisesRegex(ValueError, "output identity"):
            deb.build(self.staged, wrong, 1_234_567_890)
        self.build()
        with self.assertRaisesRegex(ValueError, "output identity"):
            self.build()


if __name__ == "__main__":
    unittest.main()

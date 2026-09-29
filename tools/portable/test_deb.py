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
            self.add(inputs, name, kind, prefix + f"fixture-{index}".encode())
        self.add(inputs, "usr/lib/linux-vst-bridge/proxy/Test.so", "proxy", b"\x7fELFproxy")
        self.add(inputs, "usr/lib/linux-vst-bridge/self-test/Test.vst3", "fixture", b"MZfixture")
        self.add(inputs, "usr/lib/linux-vst-bridge/profiles/test.json", "profile", b"{}")
        self.add(inputs, "usr/share/doc/linux-vst-bridge-beta/licenses/Bridge.txt", "license", b"license")
        spec = {"schema": 1, "version": "0.1.0beta1", "source_head": "a" * 40,
                "source_tree": "b" * 40, "operator_schema": 12,
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
        data = subprocess.check_output(["dpkg-deb", "-c", str(self.package)], text=True)
        self.assertIn("usr/share/linux-vst-bridge/pkg0-manifest.json", data)
        self.assertIn("usr/share/applications/linux-audio-compatibility-manager.desktop", data)

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
            ("dependency", deb.control_bytes({"version": "0.1.0beta1", "pkgrel": 1}).replace(
                b"libc6 (>= 2.39)", b"libc6"), None),
            ("script", deb.control_bytes({"version": "0.1.0beta1", "pkgrel": 1}), b"exit 0\n"),
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

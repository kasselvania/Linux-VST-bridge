"""File-roster and fail-closed binary-package assembly tests."""
import hashlib
import json
import pathlib
import io
import subprocess
import tarfile
import tempfile
import unittest

import assemble
import verify_package


def package_archive(staged, destination, *, pkginfo=None, extra=()):
    if pkginfo is None:
        version = json.loads((staged / "RELEASE_MANIFEST.json").read_bytes())["version"]
        pkginfo = (f"pkgname = linux-vst-bridge-beta\npkgver = {version}-1\narch = x86_64\n"
                   + "".join(f"depend = {dep}\n" for dep in sorted(verify_package.DEPENDENCIES)))
    raw = destination.with_suffix(".tar")
    with tarfile.open(raw, "w", format=tarfile.PAX_FORMAT) as target:
        for name, data in [(".PKGINFO", pkginfo.encode()),
                           (".BUILDINFO", b"buildenv = fixture\n"),
                           (".MTREE", b"fixture metadata\n"), *extra]:
            entry = tarfile.TarInfo(name)
            entry.size = len(data)
            entry.mode = 0o644
            target.addfile(entry, io.BytesIO(data))
        with tarfile.open(staged / "payload.tar") as source:
            for entry in source:
                target.addfile(entry, source.extractfile(entry) if entry.isfile() else None)
    with destination.open("wb") as output:
        subprocess.run(["zstd", "-q", "-c", str(raw)], stdout=output, check=True)
    raw.unlink()
    return destination


class PackageAssembly(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = pathlib.Path(self.temp.name)
        self.inputs = self.root / "inputs"
        self.inputs.mkdir()
        self.files = []
        for n, (name, kind) in enumerate(assemble.REQUIRED.items()):
            data = b"\x7fELF" + bytes(20) if kind in ("manager", "frontend") else b"MZ" + bytes(20) if kind == "windows_host" else f"test-{kind}".encode()
            self.add_file(name, kind, data, n)
        self.add_file("usr/lib/linux-vst-bridge/proxy/SelfTest.so", "proxy", b"\x7fELF" + bytes(20), 101)
        self.add_file("usr/lib/linux-vst-bridge/self-test/SelfTest.vst3", "fixture", b"MZ" + bytes(20), 102)
        self.add_file("usr/lib/linux-vst-bridge/self-test/again.vst3/Contents/Resources/again.uidesc",
                      "fixture_resource", b"reference editor resource", 105)
        self.add_file("usr/lib/linux-vst-bridge/profiles/self-test.json", "profile", b"{}", 103)
        self.add_file("usr/share/doc/linux-vst-bridge-beta/licenses/Bridge.txt", "license", b"license", 104)
        self.spec = {"schema": 1, "version": "0.1.0beta1", "source_head": "a" * 40,
                     "source_tree": "b" * 40, "operator_schema": 10,
                     "external_runtime": {"id": "exact-proton-slr", "manifest_sha256": "c" * 64},
                     "files": self.files}

    def add_file(self, name, kind, data, n):
        source = self.inputs / str(n)
        source.write_bytes(data)
        item = {"destination": name, "source": str(source),
                           "sha256": hashlib.sha256(data).hexdigest(), "kind": kind,
                           "component": "fixture", "mode": "0555" if kind in ("manager", "frontend", "proxy") else "0444"}
        if kind in ("manager", "frontend"):
            item.update(build_head="a" * 40, build_tree="b" * 40)
        self.files.append(item)

    def test_binary_roster_and_external_runtime_contract(self):
        out = self.root / "out"
        manifest = assemble.build(self.spec, out, 1234567890)
        self.assertEqual(manifest["source_head"], "a" * 40)
        with tarfile.open(out / "payload.tar") as tar:
            names = set(tar.getnames())
            self.assertEqual(names, {x["destination"] for x in self.files} | {assemble.ADOPTION_MANIFEST})
            adoption = json.loads(tar.extractfile(assemble.ADOPTION_MANIFEST).read())
            self.assertEqual(adoption["operator_schema"], 10)
            self.assertEqual(adoption["package"], "linux-vst-bridge-beta")
            self.assertEqual(adoption["version"], self.spec["version"])
            self.assertEqual(adoption["external_runtime"], self.spec["external_runtime"])
            self.assertFalse(any("/runtime/" in x for x in names))
            self.assertFalse(any(x.endswith((".rs", ".cpp", ".py")) or ".git" in x for x in names))
        self.assertIn(manifest["payload_sha256"], (out / "PKGBUILD").read_text())
        self.assertNotIn(str(self.root), (out / "RELEASE_MANIFEST.json").read_text())
        self.assertFalse((out / "linux-vst-bridge-beta.install").exists())
        self.assertNotIn("install=", (out / "PKGBUILD").read_text())
        package = package_archive(out, self.root / "fixture.pkg.tar.zst")
        self.assertEqual(verify_package.verify(package, manifest, True)["files"], len(self.files) + 1)

    def test_release_roster_drift_refuses(self):
        out = self.root / "roster"
        manifest = assemble.build(self.spec, out, 1234567890)
        manifest["files"][0]["sha256"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "package bytes differ"):
            verify_package.verify(package_archive(out, self.root / "drift.pkg.tar.zst"), manifest, True)

    def test_package_metadata_and_install_hook_refusals(self):
        out = self.root / "metadata"
        manifest = assemble.build(self.spec, out, 1234567890)
        version = self.spec["version"]
        normal = (f"pkgname = linux-vst-bridge-beta\npkgver = {version}-1\narch = x86_64\n"
                  + "".join(f"depend = {dep}\n" for dep in sorted(verify_package.DEPENDENCIES)))
        cases = [
            ("hook", normal, ((".INSTALL", b"post_install() {}"),)),
            ("name", normal.replace("pkgname = linux-vst-bridge-beta", "pkgname = foreign"), ()),
            ("version", normal.replace(f"pkgver = {version}-1", "pkgver = 99-1"), ()),
            ("architecture", normal.replace("arch = x86_64", "arch = aarch64"), ()),
            ("dependencies", normal.replace("depend = glibc\n", ""), ()),
            ("duplicate", normal, ((".PKGINFO", normal.encode()),)),
            ("unexpected", normal, ((".EXTRA", b"x"),)),
            ("duplicate_optional", normal, ((".MTREE", b"x"),)),
        ]
        for name, info, extra in cases:
            with self.subTest(name=name):
                package = package_archive(out, self.root / f"{name}.pkg.tar.zst",
                                          pkginfo=info, extra=extra)
                with self.assertRaises(ValueError):
                    verify_package.verify(package, manifest, True)

    def test_missing_product_payload_or_runtime_identity_refuses(self):
        for kind in ("profile", "fixture", "license"):
            with self.subTest(kind=kind):
                spec = {**self.spec, "files": [x for x in self.files if x["kind"] != kind]}
                with self.assertRaisesRegex(ValueError, "absent"):
                    assemble.validate(spec)
        bad = {**self.spec, "external_runtime": {"id": "runtime", "manifest_sha256": "bad"}}
        with self.assertRaisesRegex(ValueError, "external runtime"):
            assemble.validate(bad)

    def test_manager_frontend_build_generation_must_match(self):
        self.files[1]["build_head"] = "c" * 40
        with self.assertRaisesRegex(ValueError, "paired"):
            assemble.validate(self.spec)

    def test_fixture_resources_are_packaged_without_executable_claim(self):
        out = self.root / "resources"
        manifest = assemble.build(self.spec, out, 1234567890)
        row = next(x for x in manifest["files"] if x["kind"] == "fixture_resource")
        self.assertEqual(row["size"], len(b"reference editor resource"))
        module = next(x for x in self.files if x["kind"] == "fixture")
        module["source"] = next(x["source"] for x in self.files if x["kind"] == "fixture_resource")
        module["sha256"] = hashlib.sha256(b"reference editor resource").hexdigest()
        with self.assertRaisesRegex(ValueError, "Windows executable format"):
            assemble.build(self.spec, self.root / "bad-module", 1234567890)

    def test_mismatched_digest_and_source_bytes_refuse(self):
        self.files[0]["sha256"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "digest differs"):
            assemble.build(self.spec, self.root / "bad", 1234567890)
        name = self.files[0]["source"]
        self.assertFalse((self.root / "bad" / "RELEASE_MANIFEST.json").exists())
        self.assertTrue(pathlib.Path(name).exists())

    def test_source_and_escape_destinations_refuse(self):
        for name in ("usr/lib/linux-vst-bridge/proxy/foo.cpp", "usr/../etc/passwd", "/usr/bin/manager"):
            with self.subTest(name=name):
                item = dict(self.files[-3])
                item["destination"] = name
                item["kind"] = "proxy"
                spec = {**self.spec, "files": self.files[:-3] + [item] + self.files[-2:]}
                with self.assertRaises(ValueError):
                    assemble.validate(spec)

    def test_secret_like_bytes_refuse(self):
        source = pathlib.Path(self.files[0]["source"])
        data = b"\x7fELF-----BEGIN PRIVATE KEY-----"
        source.write_bytes(data)
        self.files[0]["sha256"] = hashlib.sha256(data).hexdigest()
        with self.assertRaisesRegex(ValueError, "secret-like"):
            assemble.build(self.spec, self.root / "secret", 1234567890)
        self.assertFalse((self.root / "secret").exists())

    def test_runtime_files_are_not_in_the_package(self):
        self.add_file("usr/lib/linux-vst-bridge/runtime/proton/bin/wine", "runtime", b"runtime", 110)
        with self.assertRaises(ValueError):
            assemble.validate(self.spec)


if __name__ == "__main__":
    unittest.main()

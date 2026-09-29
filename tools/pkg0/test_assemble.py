"""File-roster and fail-closed binary-package assembly tests."""
import hashlib
import json
import pathlib
import io
import copy
import subprocess
import tarfile
import tempfile
import unittest
import zipfile

import assemble
import verify_package


def package_archive(staged, destination, *, pkginfo=None, extra=()):
    if pkginfo is None:
        release = json.loads((staged / "RELEASE_MANIFEST.json").read_bytes())
        pkginfo = (f"pkgname = linux-vst-bridge-beta\npkgver = {release['version']}-{release['pkgrel']}\narch = x86_64\n"
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


def changed_adoption_package(staged, output, mutate):
    """Keep the outer release roster self-consistent while changing intake authority."""
    output.mkdir()
    release = json.loads((staged / "RELEASE_MANIFEST.json").read_bytes())
    with tarfile.open(staged / "payload.tar") as source, tarfile.open(
            output / "payload.tar", "w", format=tarfile.PAX_FORMAT) as target:
        for entry in source:
            data = source.extractfile(entry).read()
            if entry.name == assemble.ADOPTION_MANIFEST:
                adoption = json.loads(data)
                mutate(adoption)
                data = assemble.canonical(adoption)
                entry.size = len(data)
                row = next(row for row in release["files"]
                           if row["destination"] == assemble.ADOPTION_MANIFEST)
                row["sha256"] = hashlib.sha256(data).hexdigest()
                row["size"] = len(data)
            target.addfile(entry, io.BytesIO(data))
    (output / "RELEASE_MANIFEST.json").write_bytes(assemble.canonical(release))
    return package_archive(output, output / "changed.pkg.tar.zst"), release


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
                     "source_tree": "b" * 40, "operator_schema": 12,
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

    def add_kit(self, *, source_commit=None):
        contents = {
            "CMakeLists.txt": b"owned native build",
            "native-vst3-proxy/CMakeLists.txt": b"owned proxy target",
            "native-vst3-proxy/source/processor.cpp": b"owned processor",
            "native-vst3-proxy/source/factory.cpp": b"owned factory",
            "native-vst3-proxy/source/processor.h": b"owned processor header",
            "native-vst3-proxy/include/ap2_backend.h": b"owned backend header",
            "vst-state/stream.h": b"owned state header",
            "cmake/HP0ModernGcc.cmake": b"owned compiler policy",
            "cmake/HP0Vst3SdkLock.cmake": b"owned SDK policy",
            "libap2_backend.a": b"!<arch>\n",
            "runtime/host.exe": (self.inputs / "4").read_bytes(),
            "runtime/host-source-manifest.json": (self.inputs / "5").read_bytes(),
            "tools/mf3/native_builder.py": b"owned builder",
            "tools/ap8_descriptor.py": b"owned generator",
        }
        recipe = {"schema": 2, "source_commit": source_commit or self.spec["source_head"],
                  "sdk": assemble.KIT_SDK, "sdk_runtime": assemble.KIT_SDK_RUNTIME,
                  "files": {name: hashlib.sha256(data).hexdigest()
                            for name, data in contents.items()}}
        archive = io.BytesIO()
        with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as target:
            for name, data in contents.items():
                target.writestr(name, data)
            target.writestr("recipe.json", json.dumps(recipe))
        self.add_file(assemble.KIT_DESTINATION, "preparation_kit", archive.getvalue(), 106)
        self.spec["schema"] = 2
        return contents

    def test_package_kit_is_bound_to_adoption_and_host_pair(self):
        self.add_kit()
        out = self.root / "with-kit"
        release = assemble.build(self.spec, out, 1234567890)
        with tarfile.open(out / "payload.tar") as package:
            adoption = json.loads(package.extractfile(assemble.ADOPTION_MANIFEST).read())
        self.assertEqual(adoption["schema"], 2)
        self.assertEqual(adoption["files"][-1]["name"], "preparation-kit.zip")
        self.assertEqual(adoption["files"][-1]["sha256"],
                         next(item for item in self.files if item["kind"] == "preparation_kit")["sha256"])
        package = package_archive(out, self.root / "with-kit.pkg.tar.zst")
        self.assertEqual(verify_package.verify(package, release, True)["files"], len(self.files) + 1)

    def test_package_adoption_manifest_binds_every_selected_artifact(self):
        self.add_kit()
        out = self.root / "adoption"
        assemble.build(self.spec, out, 1234567890)
        changes = {
            "manager": lambda a: a["files"][0].update(sha256="0" * 64),
            "kit": lambda a: a["files"][-1].update(sha256="0" * 64),
            "missing": lambda a: a["files"].pop(1),
            "duplicate": lambda a: a["files"].append(copy.deepcopy(a["files"][0])),
            "extra": lambda a: a["files"].append(
                {"name": "foreign", "sha256": "0" * 64, "size": 1}),
            "operator": lambda a: a.update(operator_schema=11),
        }
        for name, mutate in changes.items():
            with self.subTest(name=name):
                package, release = changed_adoption_package(
                    out, self.root / f"changed-{name}", mutate)
                with self.assertRaisesRegex(ValueError, "package adoption"):
                    verify_package.verify(package, release, True)

    def test_incomplete_or_wrong_sdk_kit_refuses(self):
        self.add_kit()
        kit = next(item for item in self.files if item["kind"] == "preparation_kit")
        original = pathlib.Path(kit["source"]).read_bytes()
        for name, changed in [
            ("missing_target", lambda files, recipe: (
                files.pop("native-vst3-proxy/CMakeLists.txt"),
                recipe["files"].pop("native-vst3-proxy/CMakeLists.txt"))),
            ("missing_processor", lambda files, recipe: (
                files.pop("native-vst3-proxy/source/processor.cpp"),
                recipe["files"].pop("native-vst3-proxy/source/processor.cpp"))),
            ("wrong_sdk", lambda files, recipe: recipe.update(sdk="other")),
            ("wrong_runtime", lambda files, recipe: recipe.update(sdk_runtime="other")),
        ]:
            with self.subTest(name=name):
                with zipfile.ZipFile(io.BytesIO(original)) as archive:
                    files = {key: archive.read(key) for key in archive.namelist()
                             if key != "recipe.json"}
                    recipe = json.loads(archive.read("recipe.json"))
                changed(files, recipe)
                data = io.BytesIO()
                with zipfile.ZipFile(data, "w") as archive:
                    for key, value in files.items():
                        archive.writestr(key, value)
                    archive.writestr("recipe.json", json.dumps(recipe))
                pathlib.Path(kit["source"]).write_bytes(data.getvalue())
                kit["sha256"] = hashlib.sha256(data.getvalue()).hexdigest()
                with self.assertRaisesRegex(ValueError, "preparation kit"):
                    assemble.build(self.spec, self.root / f"bad-{name}", 1234567890)

    def test_source_backed_kit_requires_the_complete_clean_git_tree(self):
        contents = self.add_kit()
        source = self.root / "source"
        for name, data in contents.items():
            if name in assemble.KIT_GENERATED:
                continue
            path = source / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
        subprocess.run(["git", "init", "-q", str(source)], check=True)
        subprocess.run(["git", "-C", str(source), "add", "."], check=True)
        subprocess.run(["git", "-C", str(source), "-c", "user.name=Fixture",
                        "-c", "user.email=fixture@example.invalid", "commit", "-qm",
                        "source kit"], check=True)
        head = subprocess.check_output(["git", "-C", str(source), "rev-parse", "HEAD"],
                                       text=True).strip()
        tree = subprocess.check_output(["git", "-C", str(source), "rev-parse", "HEAD^{tree}"],
                                       text=True).strip()
        kit = next(item for item in self.files if item["kind"] == "preparation_kit")
        with zipfile.ZipFile(kit["source"]) as archive:
            recipe = json.loads(archive.read("recipe.json"))
            files = {name: archive.read(name) for name in recipe["files"]}
        recipe["source_commit"] = head
        data = io.BytesIO()
        with zipfile.ZipFile(data, "w") as archive:
            for name, value in files.items():
                archive.writestr(name, value)
            archive.writestr("recipe.json", json.dumps(recipe))
        assemble.verify_kit_source(data.getvalue(), source, head, tree)
        changed = dict(files)
        changed["native-vst3-proxy/source/processor.cpp"] = b"other processor"
        recipe["files"]["native-vst3-proxy/source/processor.cpp"] = hashlib.sha256(
            changed["native-vst3-proxy/source/processor.cpp"]).hexdigest()
        bad = io.BytesIO()
        with zipfile.ZipFile(bad, "w") as archive:
            for name, value in changed.items():
                archive.writestr(name, value)
            archive.writestr("recipe.json", json.dumps(recipe))
        with self.assertRaisesRegex(ValueError, "source bytes differ"):
            assemble.verify_kit_source(bad.getvalue(), source, head, tree)
        (source / "native-vst3-proxy/source/processor.cpp").write_bytes(b"local drift")
        with self.assertRaisesRegex(ValueError, "source head/tree differs"):
            assemble.verify_kit_source(data.getvalue(), source, head, tree)

    def test_kit_schema_source_and_host_mismatch_refuse(self):
        with self.assertRaisesRegex(ValueError, "required"):
            assemble.validate({**self.spec, "schema": 2})
        self.add_kit(source_commit="0" * 40)
        with self.assertRaisesRegex(ValueError, "preparation kit recipe"):
            assemble.build(self.spec, self.root / "wrong-source", 1234567890)
        self.files.pop()
        self.add_kit()
        self.spec["files"] = self.files
        host = next(item for item in self.files if item["kind"] == "windows_host")
        changed_host = b"MZdifferent host"
        pathlib.Path(host["source"]).write_bytes(changed_host)
        host["sha256"] = hashlib.sha256(changed_host).hexdigest()
        with self.assertRaisesRegex(ValueError, "preparation kit host pair"):
            assemble.build(self.spec, self.root / "wrong-host", 1234567890)

    def test_old_package_schema_cannot_smuggle_a_kit(self):
        self.add_kit()
        self.spec["schema"] = 1
        with self.assertRaisesRegex(ValueError, "preparation kit required"):
            assemble.validate(self.spec)

    def test_binary_roster_and_external_runtime_contract(self):
        out = self.root / "out"
        manifest = assemble.build(self.spec, out, 1234567890)
        self.assertEqual(manifest["source_head"], "a" * 40)
        with tarfile.open(out / "payload.tar") as tar:
            names = set(tar.getnames())
            self.assertEqual(names, {x["destination"] for x in self.files} | {assemble.ADOPTION_MANIFEST})
            adoption = json.loads(tar.extractfile(assemble.ADOPTION_MANIFEST).read())
            self.assertEqual(adoption["operator_schema"], 12)
            self.assertEqual(adoption["package"], "linux-vst-bridge-beta")
            self.assertEqual(adoption["version"], self.spec["version"])
            self.assertEqual(adoption["pkgrel"], 1)
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
            ("pkgrel", normal.replace(f"pkgver = {version}-1", f"pkgver = {version}-2"), ()),
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
        bad_release = {**manifest, "pkgrel": 2}
        package = package_archive(out, self.root / "bad-release.pkg.tar.zst")
        with self.assertRaisesRegex(ValueError, "release manifest schema"):
            verify_package.verify(package, bad_release, True)

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

"""File-roster and fail-closed binary-package assembly tests."""
import hashlib
import json
import pathlib
import io
import copy
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest
import zipfile

import assemble
import verify_package


def launch_provenance(binary):
    text = b"Retained runtime copyright and license text\n"
    notices = b"Retained build notices\n\n=== notices/compiler-LICENSE.txt ===\n" + text + b"\n"
    origin = dict(schema=1, component="installer_launch", artifact_sha256=assemble.sha(binary),
                  sources={name: assemble.sha((name + "\n").encode()) for name in assemble.LAUNCH_SOURCES},
                  build=dict(target="x86_64-windows-gnu", compiler="zig c++", compiler_version="0.14.1",
                             compiler_sha256="ab" * 32, flags=list(assemble.LAUNCH_FLAGS),
                             invocation_sha256="cd" * 32,
                             producer={"source": assemble.LAUNCH_PRODUCER,
                                       "sha256": assemble.sha(b"retained producer recipe")},
                             link_inputs=[{"file": "launch.obj", "sha256": "ef" * 32}],
                             notices=[{"file": "compiler-LICENSE.txt", "sha256": assemble.sha(text)}],
                             notices_sha256=assemble.sha(notices)))
    return origin, notices


def staged_python_minor(staged):
    with tarfile.open(staged / "payload.tar") as archive:
        if assemble.source_supervisor(archive.getnames()):
            return assemble.SOURCE_PYTHON_RANGE
        headers = {name: archive.extractfile(name).read(16)
                   for name in assemble.SUPERVISOR_PATHS}
    return assemble.supervisor_python_minor(headers)


def package_archive(staged, destination, *, pkginfo=None, extra=()):
    if pkginfo is None:
        release = json.loads((staged / "RELEASE_MANIFEST.json").read_bytes())
        pkginfo = (f"pkgname = linux-vst-bridge-beta\npkgver = {release['version']}-{release['pkgrel']}\narch = x86_64\n"
                   + "".join(f"depend = {dep}\n" for dep in sorted(
                       assemble.package_dependencies(staged_python_minor(staged)))))
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
            if kind in ("supervisor", "ownership"):
                data = bytes.fromhex("2b0e0d0a") + bytes(12) + f"test-{kind}".encode()
            else:
                data = (b"\x7fELF" + bytes(20) if kind in ("manager", "frontend")
                        else b"MZ" + bytes(20) if kind == "windows_host"
                        else f"test-{kind}".encode())
            self.add_file(name, kind, data, n)
        self.add_file("usr/lib/linux-vst-bridge/proxy/SelfTest.so", "proxy", b"\x7fELF" + bytes(20), 101)
        self.add_file("usr/lib/linux-vst-bridge/self-test/SelfTest.vst3", "fixture", b"MZ" + bytes(20), 102)
        self.add_file("usr/lib/linux-vst-bridge/self-test/again.vst3/Contents/Resources/again.uidesc",
                      "fixture_resource", b"reference editor resource", 105)
        self.add_file("usr/lib/linux-vst-bridge/profiles/self-test.json", "profile", b"{}", 103)
        self.add_file("usr/share/doc/linux-vst-bridge-beta/licenses/Bridge.txt", "license", b"license", 104)
        self.spec = {"schema": 1, "version": "0.1.0beta1", "source_head": "a" * 40,
                     "source_tree": "b" * 40, "operator_schema": assemble.declared_operator_schema(),
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

    def source_pair(self):
        self.add_kit()
        self.spec["schema"] = 3
        for row in self.files:
            if row["kind"] in ("supervisor", "ownership"):
                row["destination"] = row["destination"][:-1]
                data = b"# exact supervisor fixture\nVALUE = 1\n"
                pathlib.Path(row["source"]).write_bytes(data)
                row["sha256"] = hashlib.sha256(data).hexdigest()

    def test_location_capability_is_bound_to_exact_signed_helper(self):
        self.source_pair()
        helper = next(row for row in self.files if row["kind"] == "supervisor")
        data = pathlib.Path(helper["source"]).read_bytes() + b"LIBRARY_LOCATIONS_PLAN_SCHEMA = 1\n"
        pathlib.Path(helper["source"]).write_bytes(data)
        helper["sha256"] = hashlib.sha256(data).hexdigest()
        output = self.root / "locations-staged"
        manifest = assemble.build(self.spec, output, 1_234_567_890)
        with tarfile.open(output / "payload.tar") as archive:
            adopted = json.load(archive.extractfile(assemble.ADOPTION_MANIFEST))
        self.assertEqual(adopted["library_locations"],
                         {"schema": 1, "supervisor_sha256": helper["sha256"]})
        package = package_archive(output, self.root / "locations.pkg.tar.zst")
        verify_package.verify(package, manifest, structure_only=True)
        expected = {row["destination"]: row for row in manifest["files"]}
        adopted["library_locations"]["supervisor_sha256"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "library location capability"):
            verify_package.verify_adoption(adopted, manifest, expected)

    def test_old_or_unknown_location_helpers_do_not_get_current_capability(self):
        self.assertIsNone(assemble.library_locations_capability(b"VALUE = 1\n"))
        for value in ("True", "2", "'1'"):
            with self.assertRaisesRegex(ValueError, "supervisor capability"):
                assemble.library_locations_capability(("LIBRARY_LOCATIONS_PLAN_SCHEMA = " + value).encode())

    def test_source_range_is_the_bootstrap_range_everywhere(self):
        root = pathlib.Path(__file__).resolve().parents[2]
        bootstrap = (root / "bridge-manager/runtime/bootstrap.py").read_text()
        bounds = re.search(r"not \((\d+), (\d+)\) <= sys\.version_info\[:2\] < \((\d+), (\d+)\):", bootstrap)
        self.assertIsNotNone(bounds, "the bootstrap's range check changed shape")
        low, ceiling = f"{bounds[1]}.{bounds[2]}", f"{bounds[3]}.{bounds[4]}"
        declared = f"CPython >={low},<{ceiling}"
        self.assertIn(f'"supervisor requires host {declared}"', bootstrap)
        self.assertIn(f'pub const SOURCE_PYTHON_RANGE: &str = "{declared}";',
                      (root / "crates/service/src/supervisor.rs").read_text())
        self.assertEqual(assemble.source_python_bounds(assemble.SOURCE_PYTHON_RANGE), (low, ceiling))
        self.assertEqual(assemble.package_dependencies(assemble.SOURCE_PYTHON_RANGE)[-2:],
                         (f"python>={low}", f"python<{ceiling}"))
        sys.path.append(str(root / "tools/portable"))
        import deb
        self.assertEqual(deb.python_abi({path: b"" for path in assemble.SOURCE_SUPERVISOR_PATHS}),
                         f"python3 (>= {low}), python3 (<< {ceiling})")

    def test_verifier_accepts_each_declared_source_range_and_nothing_else(self):
        manifest = {"version": "0.1.0beta1", "pkgrel": 1}
        info = self.root / ".PKGINFO"
        def write(dependencies):
            info.write_text("pkgname = linux-vst-bridge-beta\npkgver = 0.1.0beta1-1\narch = x86_64\n"
                            + "".join(f"depend = {dependency}\n" for dependency in dependencies))
        for python_range in assemble.SOURCE_PYTHON_RANGES:
            write(assemble.package_dependencies(python_range))
            verify_package.package_info(info, manifest, assemble.SOURCE_PYTHON_RANGES)
        for ceiling in ("3.17", "3.14"):
            write((*assemble.DEPENDENCIES, "python>=3.13", f"python<{ceiling}"))
            with self.assertRaisesRegex(ValueError, "dependencies differ"):
                verify_package.package_info(info, manifest, assemble.SOURCE_PYTHON_RANGES)
        # Debian packages accept the same ranges, Test225's first.
        sys.path.append(str(pathlib.Path(__file__).resolve().parents[2] / "tools/portable"))
        import deb
        self.assertEqual(deb.python_abis({path: b"" for path in assemble.SOURCE_SUPERVISOR_PATHS}),
                         ("python3 (>= 3.13), python3 (<< 3.15)", "python3 (>= 3.13), python3 (<< 3.16)"))

    def test_source_generation_has_only_source_and_declared_range(self):
        self.source_pair()
        output = self.root / "source-staged"
        assemble.build(self.spec, output, 1_234_567_890)
        self.assertIn("'python>=3.13' 'python<3.16'", (output / "PKGBUILD").read_text())
        with tarfile.open(output / "payload.tar") as archive:
            adoption = json.load(archive.extractfile(assemble.ADOPTION_MANIFEST))
            self.assertEqual(adoption["schema"], 3)
            for name in assemble.SOURCE_SUPERVISOR_PATHS:
                self.assertIn(name, archive.getnames())
                self.assertNotIn(name + 'c', archive.getnames())
        package = package_archive(output, self.root / "source.pkg.tar.zst")
        verify_package.verify(package, json.loads((output / "RELEASE_MANIFEST.json").read_bytes()), structure_only=True)

    def test_source_exception_is_exact_and_mixed_payloads_refuse(self):
        self.source_pair()
        row = next(row for row in self.files if row["kind"] == "ownership")
        row["destination"] += 'c'
        with self.assertRaisesRegex(ValueError, "pair missing or mixed"):
            assemble.validate(self.spec)
        for name, kind in [('usr/lib/linux-vst-bridge/self-test/other.py', 'fixture_resource'),
                           ('usr/lib/linux-vst-bridge/supervisor/other.py', 'ownership')]:
            with self.assertRaises(ValueError):
                assemble.validate_file_role(name, kind, '0444', 'fixture')

    def set_supervisor_header(self, kind, magic, *, flags=0):
        item = next(row for row in self.files if row["kind"] == kind)
        path = pathlib.Path(item["source"])
        data = magic + flags.to_bytes(4, "little") + bytes(8) + b"test-bytecode"
        path.write_bytes(data)
        item["sha256"] = hashlib.sha256(data).hexdigest()

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
        self.assertEqual(verify_package.verify(package, release, True)["files"], len(self.files) + 2)

    def test_reusable_engine_kit_packages_and_rejects_inconsistent_metadata(self):
        files = {
            "libap2_backend.a": b"!<arch>\n",
            "runtime/host.exe": (self.inputs / "4").read_bytes(),
            "runtime/host-source-manifest.json": (self.inputs / "5").read_bytes(),
            "tools/mf3/native_builder.py": b"owned builder",
            "tools/ap8_descriptor.py": b"owned generator",
            "prebuilt/engine.so": b"\x7fELFreusable engine fixture",
            **{f"licenses/{name}.txt": b"retained license"
               for name in ("vst3sdk", "base", "pluginterfaces", "public.sdk")},
        }
        index = dict(schema=3, engine="prebuilt/engine.so",
                     engine_sha256=assemble.sha(files["prebuilt/engine.so"]),
                     descriptor_schema=1, maximum_bridge_frames=1024,
                     audio_completion_contract=1, loaded_engine_admission_contract=1,
                     native_sources={})

        def archive_bytes(selected, extra=None):
            contents = {**files, **(extra or {}),
                        "prebuilt/index.json": json.dumps(selected).encode()}
            recipe = dict(schema=4, source_commit=self.spec["source_head"],
                          sdk=assemble.KIT_SDK, sdk_runtime=assemble.KIT_SDK_RUNTIME,
                          files={name: assemble.sha(data) for name, data in contents.items()})
            data = io.BytesIO()
            with zipfile.ZipFile(data, "w") as archive:
                for name, value in contents.items():
                    archive.writestr(name, value)
                archive.writestr("recipe.json", json.dumps(recipe))
            return data.getvalue()

        self.add_file(assemble.KIT_DESTINATION, "preparation_kit", archive_bytes(index), 106)
        self.spec["schema"] = 2
        out = self.root / "reusable-kit"
        release = assemble.build(self.spec, out, 1234567890)
        package = package_archive(out, self.root / "reusable.pkg.tar.zst")
        self.assertEqual(verify_package.verify(package, release, True)["files"], len(self.files) + 2)
        assemble.verify_kit(archive_bytes({**index, "audio_completion_contract": 1}),
                            self.spec["source_head"], assemble.sha(files["runtime/host.exe"]),
                            assemble.sha(files["runtime/host-source-manifest.json"]))
        for change in ({"engine_sha256": "0" * 64}, {"descriptor_schema": 2},
                       {"loaded_engine_admission_contract": 0},
                       {"maximum_bridge_frames": 64}, {"proxies": []},
                       {"audio_completion_contract": 2}, {"audio_completion_contract": True}):
            with self.subTest(change=change), self.assertRaisesRegex(ValueError, "reusable engine index"):
                assemble.verify_kit(archive_bytes({**index, **change}), self.spec["source_head"],
                                    assemble.sha(files["runtime/host.exe"]),
                                    assemble.sha(files["runtime/host-source-manifest.json"]))
        with self.assertRaisesRegex(ValueError, "prebuilt kit entry"):
            assemble.verify_kit(archive_bytes(index, {"unowned": b"unexpected"}),
                                self.spec["source_head"], assemble.sha(files["runtime/host.exe"]),
                                assemble.sha(files["runtime/host-source-manifest.json"]))
        launch=b"MZsource-owned launch fixture"
        origin, notices=launch_provenance(launch)
        component={"runtime/installer-launch.exe":launch,
            "runtime/installer-launch-source.json":json.dumps(origin).encode(),
            "licenses/installer-launch.txt":notices}
        assemble.verify_kit(archive_bytes(index,component),self.spec["source_head"],
            assemble.sha(files["runtime/host.exe"]),assemble.sha(files["runtime/host-source-manifest.json"]))
        with self.assertRaisesRegex(ValueError,"launch component roster"):
            assemble.verify_kit(archive_bytes(index,{"runtime/installer-launch.exe":launch}),self.spec["source_head"],
                assemble.sha(files["runtime/host.exe"]),assemble.sha(files["runtime/host-source-manifest.json"]))
        bad={**component,"runtime/installer-launch-source.json":json.dumps({**origin,"artifact_sha256":"00"*32}).encode()}
        with self.assertRaisesRegex(ValueError,"launch component provenance"):
            assemble.verify_kit(archive_bytes(index,bad),self.spec["source_head"],
                assemble.sha(files["runtime/host.exe"]),assemble.sha(files["runtime/host-source-manifest.json"]))
        for name in assemble.LAUNCH_SOURCES:
            missing=copy.deepcopy(origin);missing["sources"].pop(name)
            bad={**component,"runtime/installer-launch-source.json":json.dumps(missing).encode()}
            with self.subTest(missing=name), self.assertRaisesRegex(ValueError,"launch component provenance"):
                assemble.verify_kit(archive_bytes(index,bad),self.spec["source_head"],
                    assemble.sha(files["runtime/host.exe"]),assemble.sha(files["runtime/host-source-manifest.json"]))
        for change in ({"flags":[*assemble.LAUNCH_FLAGS,"-DNAD1_GENERATED_SERVICE"]},
                       {"compiler_path":"/private/compiler"}, {"target":"another-target"}):
            changed=copy.deepcopy(origin);changed["build"].update(change)
            bad={**component,"runtime/installer-launch-source.json":json.dumps(changed).encode()}
            with self.subTest(change=change), self.assertRaisesRegex(ValueError,"launch component provenance"):
                assemble.verify_kit(archive_bytes(index,bad),self.spec["source_head"],
                    assemble.sha(files["runtime/host.exe"]),assemble.sha(files["runtime/host-source-manifest.json"]))
        changed=copy.deepcopy(origin)
        changed["build"]["notices"][0]["sha256"]="00"*32
        for bad in ({**component,"licenses/installer-launch.txt":notices+b"changed"},
                    {**component,"runtime/installer-launch-source.json":json.dumps(changed).encode()}):
            with self.assertRaisesRegex(ValueError,"launch component notices"):
                assemble.verify_kit(archive_bytes(index,bad),self.spec["source_head"],
                    assemble.sha(files["runtime/host.exe"]),assemble.sha(files["runtime/host-source-manifest.json"]))
        raw=copy.deepcopy(origin)
        build=raw["build"];build.pop("flags");build.pop("invocation_sha256");build.pop("notices_sha256")
        build.update(arguments=["c++",*assemble.LAUNCH_FLAGS[:7],"/private/source/launch.cpp",
                                *assemble.LAUNCH_FLAGS[7:],"-o","/private/build/installer-launch.exe"],
                     compiler_path="/private/compiler/zig",generated_service_define=False,
                     combined_notices={"file":"Installer-Launch-Notices.txt","sha256":assemble.sha(notices)})
        build["link_inputs"]=[{"path":"/private/cache/launch.obj","sha256":"ef"*32}]
        build["notices"]=[{"file":"notices/compiler-LICENSE.txt","source":"/private/compiler/LICENSE",
                           "sha256":origin["build"]["notices"][0]["sha256"]}]
        raw_component={**component,"runtime/installer-launch-source.json":json.dumps(raw).encode()}
        projected=assemble.public_launch_kit(archive_bytes(index,raw_component))
        assemble.verify_kit(projected,self.spec["source_head"],assemble.sha(files["runtime/host.exe"]),
                            assemble.sha(files["runtime/host-source-manifest.json"]))
        with zipfile.ZipFile(io.BytesIO(projected)) as archive:
            public=json.loads(archive.read("runtime/installer-launch-source.json"))
            self.assertNotIn(b"/private",archive.read("runtime/installer-launch-source.json"))
            self.assertEqual(public["sources"],origin["sources"])
            self.assertEqual(public["build"]["invocation_sha256"],assemble.sha(assemble.canonical(build["arguments"])))
            for name in files:
                self.assertEqual(archive.read(name),files[name])
            self.assertEqual(archive.read("runtime/installer-launch.exe"),launch)
            self.assertEqual(archive.read("licenses/installer-launch.txt"),notices)
        self.assertEqual(assemble.public_launch_kit(projected),projected)
        build["arguments"].insert(8,"-DNAD1_GENERATED_SERVICE")
        with self.assertRaisesRegex(ValueError,"launch component production recipe"):
            assemble.public_launch_kit(archive_bytes(index,{**component,
                "runtime/installer-launch-source.json":json.dumps(raw).encode()}))
        helpers={"runtime/lvb-direct-wait.dll":b"MZ Wine builtin DLL fixture",
                 "runtime/x86_64-windows/lvb-direct-wait.dll":b"MZ Wine builtin DLL fixture",
                 "runtime/x86_64-unix/lvb-direct-wait.so":b"\x7fELFowned helper"}
        binding=dict(schema=1,abi=1,host_sha256=assemble.sha(files["runtime/host.exe"]),
                     runner_wine_revision="46b29104e3741fe23bf5e2547196a253aab88c89",
                     files={name.removeprefix("runtime/"):assemble.sha(data) for name,data in helpers.items()})
        helpers["runtime/direct-audio-helper.json"]=json.dumps(binding).encode()
        def verify_helpers(extra):
            assemble.verify_kit(archive_bytes(index,extra),self.spec["source_head"],
                                assemble.sha(files["runtime/host.exe"]),
                                assemble.sha(files["runtime/host-source-manifest.json"]))
        verify_helpers(helpers)
        for name in helpers:
            with self.subTest(missing=name),self.assertRaisesRegex(ValueError,"direct helper roster"):
                verify_helpers({key:value for key,value in helpers.items() if key!=name})
        for change in ({"host_sha256":"0"*64},{"abi":True},{"files":{}},
                       {"runner_wine_revision":"unselected"}):
            with self.subTest(binding=change),self.assertRaisesRegex(ValueError,"direct helper binding"):
                verify_helpers({**helpers,"runtime/direct-audio-helper.json":json.dumps({**binding,**change}).encode()})
        common={**binding,"schema":2,"runner_wine_revision":"c"*40,"runner_build_sha256":"d"*64}
        verify_helpers({**helpers,"runtime/direct-audio-helper.json":json.dumps(common).encode()})
        for change in ({"runner_build_sha256":"latest"},{"runner_build_sha256":None},
                       {"runner_wine_revision":"unselected"},{"runner_wine_revision":True}):
            with self.subTest(common=change),self.assertRaisesRegex(ValueError,"direct helper binding"):
                verify_helpers({**helpers,"runtime/direct-audio-helper.json":json.dumps({**common,**change}).encode()})

    def test_operator_pair_uses_exact_source_and_retained_predecessor(self):
        current = assemble.declared_operator_schema()
        self.assertEqual(self.spec["operator_schema"], current)
        predecessor = self.root / "predecessor-source"
        model = predecessor / "bridge-manager/src/operator_model.rs"
        model.parent.mkdir(parents=True)
        model.write_text("pub const OPERATOR_SCHEMA: u32 = 14;\n")
        old_spec = {**self.spec, "operator_schema": 14}
        with self.assertRaisesRegex(ValueError, "paired operator schema"):
            assemble.validate(old_spec)
        old_staged = self.root / "old-staged"
        release = assemble.build(old_spec, old_staged, 1234567890, predecessor)
        package = package_archive(old_staged, self.root / "predecessor.pkg.tar.zst")
        with self.assertRaisesRegex(ValueError, "package adoption identity"):
            verify_package.verify(package, release, True)
        self.assertEqual(verify_package.verify(package, release, True, predecessor)["files"],
                         len(self.files) + 2)
        with self.assertRaisesRegex(ValueError, "paired operator schema"):
            assemble.validate(self.spec, predecessor)
        with self.assertRaisesRegex(ValueError, "paired operator schema"):
            assemble.validate({**self.spec, "operator_schema": float(current)})

    def test_operator_schema_declaration_must_be_unique_and_bounded(self):
        for source in ("", "pub const OPERATOR_SCHEMA: u32 = 0;\n",
                       "pub const OPERATOR_SCHEMA: u32 = 4294967296;\n",
                       "pub const OPERATOR_SCHEMA: u32 = 15;\n" * 2):
            with self.subTest(source=source):
                with self.assertRaisesRegex(ValueError, "declared operator schema"):
                    assemble.parse_operator_schema(source)

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

    def test_launch_source_binding_checks_every_header_and_the_producer(self):
        source=self.root/"launch-source"
        contents={name:(name+"\n").encode() for name in assemble.LAUNCH_SOURCES}
        contents.update({assemble.LAUNCH_PRODUCER:b"retained producer recipe",
                         "tools/mf3/native_builder.py":b"owned builder",
                         "tools/ap8_descriptor.py":b"owned generator"})
        for name,data in contents.items():
            path=source/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(data)
        subprocess.run(["git","init","-q",str(source)],check=True)
        subprocess.run(["git","-C",str(source),"add","."],check=True)
        subprocess.run(["git","-C",str(source),"-c","user.name=Fixture",
                        "-c","user.email=fixture@example.invalid","commit","-qm","launch source"],check=True)
        head=subprocess.check_output(["git","-C",str(source),"rev-parse","HEAD"],text=True).strip()
        tree=subprocess.check_output(["git","-C",str(source),"rev-parse","HEAD^{tree}"],text=True).strip()
        launch=b"MZsource-owned launch fixture"
        origin,notices=launch_provenance(launch)
        def kit(selected):
            files={"prebuilt/index.json":json.dumps({"native_sources":{}}).encode(),
                   "tools/mf3/native_builder.py":contents["tools/mf3/native_builder.py"],
                   "tools/ap8_descriptor.py":contents["tools/ap8_descriptor.py"],
                   "runtime/installer-launch.exe":launch,
                   "runtime/installer-launch-source.json":json.dumps(selected).encode(),
                   "licenses/installer-launch.txt":notices}
            recipe={"schema":4,"source_commit":head,"files":{name:assemble.sha(value) for name,value in files.items()}}
            data=io.BytesIO()
            with zipfile.ZipFile(data,"w") as archive:
                for name,value in files.items():archive.writestr(name,value)
                archive.writestr("recipe.json",json.dumps(recipe))
            return data.getvalue()
        assemble.verify_kit_source(kit(origin),source,head,tree)
        for name in assemble.LAUNCH_SOURCES:
            changed=copy.deepcopy(origin);changed["sources"][name]="00"*32
            with self.subTest(changed=name), self.assertRaisesRegex(ValueError,"launch component source differs"):
                assemble.verify_kit_source(kit(changed),source,head,tree)
            changed=copy.deepcopy(origin);changed["sources"].pop(name)
            with self.subTest(missing=name), self.assertRaisesRegex(ValueError,"launch component provenance"):
                assemble.verify_kit_source(kit(changed),source,head,tree)
        changed=copy.deepcopy(origin);changed["build"]["producer"]["sha256"]="00"*32
        with self.assertRaisesRegex(ValueError,"launch component source differs"):
            assemble.verify_kit_source(kit(changed),source,head,tree)

    def test_release_backend_rebuild_rejects_stale_static_archive(self):
        if shutil.which("rustup") is None:
            self.skipTest("Rust toolchain unavailable")
        targets = subprocess.check_output(["rustup", "target", "list", "--installed"],
                                          text=True).splitlines()
        if "x86_64-unknown-linux-gnu" not in targets:
            self.skipTest("x86-64 Linux Rust target unavailable")
        self.add_kit()
        kit = next(item for item in self.files if item["kind"] == "preparation_kit")
        with self.assertRaisesRegex(ValueError, "backend source differs"):
            assemble.verify_kit_backend(pathlib.Path(kit["source"]).read_bytes(),
                                        pathlib.Path(__file__).resolve().parents[2])

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
            self.assertEqual(names, {x["destination"] for x in self.files}
                             | {assemble.ADOPTION_MANIFEST, assemble.SYSTEM_DESKTOP})
            adoption = json.loads(tar.extractfile(assemble.ADOPTION_MANIFEST).read())
            desktop = tar.extractfile(assemble.SYSTEM_DESKTOP).read()
            self.assertEqual(desktop, assemble.SYSTEM_DESKTOP_BYTES)
            self.assertIn(b"Exec=/usr/bin/linux-audio-compatibility-manager\n", desktop)
            self.assertNotEqual(assemble.SYSTEM_DESKTOP,
                                "usr/share/applications/linux-audio-compatibility-manager.desktop")
            self.assertIn(b"Name=Linux VST Bridge Setup and Updates\n", desktop)
            self.assertNotIn(b"package-adopt", desktop)
            self.assertEqual(adoption["operator_schema"], assemble.declared_operator_schema())
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
        self.assertIn("'python>=3.14' 'python<3.15'", (out / "PKGBUILD").read_text())
        for dependency in ("libxkbcommon-x11", "libxcursor", "libxi"):
            self.assertIn(repr(dependency), (out / "PKGBUILD").read_text())
        package = package_archive(out, self.root / "fixture.pkg.tar.zst")
        self.assertEqual(verify_package.verify(package, manifest, True)["files"], len(self.files) + 2)

    def test_python_313_bytecode_binds_arch_dependency_range(self):
        for kind in ("supervisor", "ownership"):
            self.set_supervisor_header(kind, bytes.fromhex("f30d0d0a"))
        out = self.root / "python313"
        manifest = assemble.build(self.spec, out, 1234567890)
        pkgbuild = (out / "PKGBUILD").read_text()
        self.assertIn("'python>=3.13' 'python<3.14'", pkgbuild)
        self.assertNotIn("python>=3.14", pkgbuild)
        package = package_archive(out, self.root / "python313.pkg.tar.zst")
        self.assertEqual(verify_package.verify(package, manifest, True)["files"], len(self.files) + 2)

    def test_mixed_unknown_or_malformed_supervisor_bytecode_refuses(self):
        cases = (
            ("mixed", bytes.fromhex("f30d0d0a"), bytes.fromhex("2b0e0d0a"), 0),
            ("unknown", bytes.fromhex("ffffffff"), bytes.fromhex("ffffffff"), 0),
            ("malformed", bytes.fromhex("2b0e0d0a"), bytes.fromhex("2b0e0d0a"), 2),
        )
        for label, session, ownership, flags in cases:
            with self.subTest(label=label):
                self.set_supervisor_header("supervisor", session, flags=flags)
                self.set_supervisor_header("ownership", ownership)
                output = self.root / f"bad-python-{label}"
                with self.assertRaisesRegex(ValueError, "supervisor Python"):
                    assemble.build(self.spec, output, 1234567890)
                self.assertFalse(output.exists())

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
                  + "".join(f"depend = {dep}\n" for dep in sorted(
                      assemble.package_dependencies(staged_python_minor(out)))))
        cases = [
            ("hook", normal, ((".INSTALL", b"post_install() {}"),)),
            ("name", normal.replace("pkgname = linux-vst-bridge-beta", "pkgname = foreign"), ()),
            ("version", normal.replace(f"pkgver = {version}-1", "pkgver = 99-1"), ()),
            ("pkgrel", normal.replace(f"pkgver = {version}-1", f"pkgver = {version}-2"), ()),
            ("architecture", normal.replace("arch = x86_64", "arch = aarch64"), ()),
            ("dependencies", normal.replace("depend = glibc\n", ""), ()),
            ("gui_dependency", normal.replace("depend = libxcursor\n", ""), ()),
            ("python_dependency", normal.replace("depend = python>=3.14\n",
                                                  "depend = python>=3.13\n"), ()),
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

    def composite_frontend(self):
        self.source_pair()
        row = next(row for row in self.files if row["kind"] == "frontend")
        del row["build_head"], row["build_tree"]
        record = {"schema": 1,
            "bridge_source": {"head": self.spec["source_head"], "tree": self.spec["source_tree"]},
            "frontend_source": {"head": "d" * 40, "tree": "e" * 40,
                "manifest": "apps/bridge/Cargo.toml", "manifest_sha256": "1" * 64,
                "cargo_lock_sha256": "2" * 64},
            "target": "x86_64-unknown-linux-gnu", "binary": "linux-audio-compatibility-manager",
            "features": ["live"], "portable_public_key": "3" * 64,
            "portable_key_class": "internal_test", "artifact_sha256": row["sha256"]}
        row["frontend_build"] = record
        compliance = next(row for row in self.files if row["kind"] == "compliance")
        data = assemble.canonical({"classification": "internal_test_only", "release_authorized": False,
                                   "frontend_build": record})
        pathlib.Path(compliance["source"]).write_bytes(data)
        compliance["sha256"] = assemble.sha(data)
        return record, compliance

    def test_composite_frontend_provenance_is_in_existing_signed_roster(self):
        record, compliance = self.composite_frontend()
        output = self.root / "composite"
        release = assemble.build(self.spec, output, 1234567890)
        self.assertEqual(release["schema"], 1)
        self.assertEqual(release["source_head"], "a" * 40)
        with tarfile.open(output / "payload.tar") as archive:
            data = archive.extractfile(compliance["destination"]).read()
            self.assertEqual(json.loads(data)["frontend_build"], record)
            signed_row = next(row for row in release["files"] if row["kind"] == "compliance")
            self.assertEqual(signed_row["sha256"], assemble.sha(data))
            adopted = json.loads(archive.extractfile(assemble.ADOPTION_MANIFEST).read())
            self.assertEqual(adopted["schema"], 3)
            selected_frontend = next(row for row in adopted["files"]
                                     if row["name"] == "linux-audio-compatibility-manager")
            self.assertEqual(selected_frontend["sha256"], record["artifact_sha256"])
            self.assertNotIn("frontend_build", adopted)
        package = package_archive(output, self.root / "composite.pkg.tar.zst")
        self.assertEqual(verify_package.verify(package, release, True)["files"], len(self.files) + 2)

    def test_composite_frontend_cannot_drift_or_omit_signed_provenance(self):
        record, compliance = self.composite_frontend()
        for key, value in (("bridge_source", {"head": "f" * 40, "tree": "b" * 40}),
                           ("artifact_sha256", "0" * 64), ("features", ["live", "samples"]),
                           ("portable_key_class", "release")):
            with self.subTest(key=key):
                old = record[key]
                record[key] = value
                with self.assertRaises(ValueError):
                    assemble.validate(self.spec)
                record[key] = old
        data = assemble.canonical({"classification": "internal_test_only", "release_authorized": False})
        pathlib.Path(compliance["source"]).write_bytes(data)
        compliance["sha256"] = assemble.sha(data)
        with self.assertRaisesRegex(ValueError, "not bound by signed compliance"):
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

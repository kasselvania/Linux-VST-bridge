#!/usr/bin/env python3
"""WholeArchive production without Wine execution or installed runtime changes."""
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import runpy
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("runtime_package", HERE / "package.py")
p = importlib.util.module_from_spec(spec)
spec.loader.exec_module(p)
companion = runpy.run_path(str(HERE / "source-companion.py"))
sys.path.insert(0, str(HERE.parent.parent / "bridge-manager/runtime"))
import session


def archive(path, entries, mode="w:gz"):
    with tarfile.open(path, mode) as output:
        for name, data in entries.items():
            item = tarfile.TarInfo(name)
            item.size = len(data)
            output.addfile(item, io.BytesIO(data))


class WholeArchiveTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="lvb-whole-archive-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.redist = self.root / "build/redist"
        self.source = self.root / "source-companion"
        self.output = self.root / "common.tar.gz"
        self.redist.mkdir(parents=True)
        for name, data in {"proton": b"launcher", "files/bin/wine": b"own native loader",
                           "third-party-notices/test.txt": b"original notice"}.items():
            path = self.redist / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
        for name in ("proton", "files/bin/wine"):
            (self.redist / name).chmod(0o755)
        (self.redist / "files/share/empty").mkdir(parents=True)
        (self.redist / "files/bin/wine32").symlink_to("wine")
        self.source.mkdir()
        (self.source / "README.md").write_text("original source collection")
        prepared = self.source / "prepared-source"
        prepared.mkdir()
        for name in companion["COMPONENTS"]:
            (prepared / (name.replace("/", "_") + ".tar.gz")).write_bytes(b"selected source")
        for name in ("proton-build", "collector-source", "piper-local-configuration"):
            (prepared / (name + ".tar.gz")).write_bytes(b"build context")
        recipe = companion["RECIPE"]
        configurations = {"i386": b"native i386 Makefile", "x86_64": b"native x64 Makefile"}
        self.build = dict(schema=1, kind="wine_build", helper_abi=1,
            recipe_sha256=companion["construction_recipe"](p.ROOT)[0],
            wine_revision=recipe["wine"]["commit"], sdk_image=recipe["sdk"],
            configuration_sha256={name: hashlib.sha256(data).hexdigest() for name, data in configurations.items()},
            invocation=dict(target="wine", source_date_epoch=recipe["proton"]["source_date_epoch"],
                            base_source_date_epoch=recipe["proton"]["source_date_epoch"],
                            build_name="source-fixture", make_arguments=["-j3", "J=3", "CONTAINER=1"]),
            helper_sources={name: p.sha(p.ROOT / name) for name in companion["HELPER_SOURCES"]},
            winebuild_sha256="1" * 64)
        self.declaration = p.canonical(self.build)
        (self.redist / "wine-build.json").write_bytes(self.declaration)
        self.build_digest = hashlib.sha256(self.declaration).hexdigest()
        companion["archive_construction"](prepared / "original-construction-recipes.tar.gz", p.ROOT, self.build)
        config = {"build/redist/wine-build.json": self.declaration}
        config.update({"build/obj-wine-" + name + "/Makefile": data for name, data in configurations.items()})
        archive(prepared / "wine-build-configuration.tar.gz", config)
        (self.source / "cargo").mkdir()
        (self.source / "cargo/selected-1.crate").write_bytes(b"exact compiled crate")
        upstream = self.source / "upstream-source"
        upstream.mkdir()
        external = json.loads((HERE / "build-inputs.json").read_bytes())
        for name in set(companion["INPUTS"]["corresponding_source"]) | (set(external) - {"onnxruntime.tgz"}):
            (upstream / name).write_bytes(b"original upstream source bytes")
        self.upstream_inputs = {path.name: dict(size=path.stat().st_size, sha256=p.sha(path))
                                for path in upstream.iterdir()}
        (self.source / "third-party-notices").mkdir()
        (self.source / "third-party-notices/test.txt").write_bytes(b"original notice")
        self.slr = self.root / "SteamLinuxRuntime_4.tar.xz"
        archive(self.slr, {name: role.encode() for role, name in p.TOOLS.items()}, "w:xz")
        self.slr_spec = dict(p.SLR, size=self.slr.stat().st_size, sha256=p.sha(self.slr))

    def source_digest(self):
        return hashlib.sha256(p.canonical(p.file_manifest(self.source, p.inventory(self.source)))).hexdigest()

    def make_package(self, **arguments):
        values = dict(build_root=self.root, source=self.source, source_digest=self.source_digest(),
                      slr_archive=self.slr, output=self.output, declaration_digest=self.build_digest)
        values.update(arguments)
        with patch.object(p, "SLR", self.slr_spec), patch.object(p, "UPSTREAM_INPUTS", self.upstream_inputs):
            return p.package(**values)

    def test_exact_payload_reproducibility_and_shared_session_binding(self):
        before = p.inventory(self.redist, links=True)
        source_before = self.source_digest()
        predecessor = self.root / "retained-predecessor"
        predecessor.write_bytes(b"unchanged predecessor")
        first = self.make_package()
        second = self.make_package(output=self.root / "again.tar.gz")
        self.assertEqual(first["sha256"], second["sha256"])
        with tarfile.open(self.output) as result:
            names = result.getnames()
            self.assertEqual(len(names), len(set(names)))
            for base, prefix in ((self.redist, ""), (self.source, "source-companion/")):
                for path in base.rglob("*"):
                    name = p.ENGINE_ROOT + "/" + prefix + path.relative_to(base).as_posix()
                    member = result.getmember(name)
                    if path.is_symlink():
                        self.assertEqual(member.linkname, str(path.readlink()))
                    elif path.is_file():
                        self.assertEqual(result.extractfile(member).read(), path.read_bytes())
                    else:
                        self.assertTrue(member.isdir())
            marker = result.extractfile(p.ENGINE_ROOT + "/native-command-session.json").read()
        self.assertEqual(json.loads(marker), dict(schema=2, kind="native_proton_command_session",
            client_sha256=hashlib.sha256(b"client_sha256").hexdigest(),
            service_sha256=hashlib.sha256(b"service_sha256").hexdigest(), wine_build_sha256=self.build_digest))
        # The real shared-session reader consumes these produced bytes.
        runtime = self.root / "shared-reader"
        runtime.mkdir()
        rows = []
        for name, data in (("native-command-session.json", marker), ("wine-build.json", self.declaration)):
            path = runtime / name
            path.write_bytes(data)
            path.chmod(0o400)
            rows.append(dict(path=str(path), sha256=p.sha(path)))
        runner = dict(proton=str(runtime / "proton"), files=rows)
        self.assertEqual(session.selected_wine_build(runner)[1], self.build_digest)
        runner["files"] = rows[:1]
        with self.assertRaisesRegex(RuntimeError, "binding missing"):
            session.selected_wine_build(runner)
        self.assertEqual(p.inventory(self.redist, links=True), before)
        self.assertEqual(self.source_digest(), source_before)
        self.assertEqual(predecessor.read_bytes(), b"unchanged predecessor")

    def test_refuses_changed_inputs_existing_output_and_unsafe_links(self):
        self.output.write_bytes(b"keep existing output")
        with self.assertRaisesRegex(ValueError, "already exists"):
            self.make_package()
        self.assertEqual(self.output.read_bytes(), b"keep existing output")
        self.output.unlink()
        for arguments, message in ((dict(source_digest="0" * 64), "Source companion identity"),
                                   (dict(declaration_digest="0" * 64), "Wine declaration")):
            with self.assertRaisesRegex(ValueError, message):
                self.make_package(**arguments)
            self.assertFalse(self.output.exists())
        (self.redist / "escape").symlink_to("../outside")
        with self.assertRaisesRegex(ValueError, "symlink escapes"):
            self.make_package()
        (self.redist / "escape").unlink()
        (self.redist / "third-party-notices/test.txt").write_bytes(b"changed notice")
        with self.assertRaisesRegex(ValueError, "notices differ"):
            self.make_package()

    def test_refuses_incomplete_source_and_forged_original_declaration(self):
        missing = self.source / "upstream-source/onnxruntime-src.tar.gz"
        missing.unlink()
        with self.assertRaisesRegex(ValueError, "upstream-source roster"):
            self.make_package()
        missing.write_bytes(b"changed source archive")
        with self.assertRaisesRegex(ValueError, "upstream input differs"):
            self.make_package()
        missing.write_bytes(b"original upstream source bytes")
        (self.redist / "wine-build.json").write_bytes(self.declaration + b"\n")
        with self.assertRaisesRegex(ValueError, "original declaration differs"):
            self.make_package(declaration_digest=p.sha(self.redist / "wine-build.json"))

    def test_refuses_changed_or_missing_slr_tools(self):
        self.slr_spec["sha256"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "fixed catalog"):
            self.make_package()
        archive(self.slr, {next(iter(p.TOOLS.values())): b"client"}, "w:xz")
        self.slr_spec.update(size=self.slr.stat().st_size, sha256=p.sha(self.slr))
        with self.assertRaisesRegex(ValueError, "tool is missing"):
            self.make_package()

    def test_refuses_malformed_construction_and_cannot_output_into_inputs(self):
        for change in (dict(schema=2), dict(helper_abi=True), dict(extra="unknown"),
                       dict(invocation=dict(self.build["invocation"], make_arguments=["/host/path"]))):
            with self.assertRaisesRegex(ValueError, "Wine construction"):
                p.read_declaration(p.canonical(dict(self.build, **change)))
        with self.assertRaisesRegex(ValueError, "Duplicate Wine declaration"):
            p.read_declaration(self.declaration.rstrip()[:-1] + b',"schema":1}')
        with self.assertRaisesRegex(ValueError, "outside its input"):
            self.make_package(output=self.source / "unexpected.tar.gz")
        self.assertFalse((self.source / "unexpected.tar.gz").exists())

    def test_slr_pin_matches_the_existing_delivery_catalog(self):
        catalog = (p.ROOT / "bridge-manager/src/runtime_delivery.rs").read_text()
        for value in (p.SLR["url"], p.SLR["sha256"], str(p.SLR["size"])):
            self.assertIn(value, catalog)


if __name__ == "__main__":
    unittest.main()

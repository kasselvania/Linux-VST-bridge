"""The sealed MSI component must bind its exact GE preimages and consumed source."""
import hashlib
import io
import json
import struct
import tarfile
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import build
import package


def pe_image(machine, imports=("kernel32.dll",), builtin=True):
    """A minimal PE with one import section; enough for the import reader."""
    magic, directories = {0x014C: (0x10b, 96), 0x8664: (0x20b, 112)}[machine]
    opt_size = directories + 16 * 8
    data = bytearray(0x400)
    data[:2] = b"MZ"
    struct.pack_into("<I", data, 0x3c, 0x80)
    if builtin:
        data[0x40:0x50] = b"Wine builtin DLL"
    data[0x80:0x84] = b"PE\0\0"
    struct.pack_into("<HH", data, 0x84, machine, 1)
    struct.pack_into("<H", data, 0x94, opt_size)
    opt = 0x98
    struct.pack_into("<H", data, opt, magic)
    struct.pack_into("<II", data, opt + directories + 8, 0x1000, 20 * (len(imports) + 1))
    section = opt + opt_size
    data[section:section + 8] = b".idata\0\0"
    struct.pack_into("<IIII", data, section + 8, 0x200, 0x1000, 0x200, 0x200)
    for index, name in enumerate(imports):
        struct.pack_into("<I", data, 0x200 + index * 20 + 12, 0x1100 + index * 32)
        encoded = name.encode()
        data[0x300 + index * 32:0x300 + index * 32 + len(encoded)] = encoded
    return bytes(data)


def add(archive, name, data):
    info = tarfile.TarInfo(name)
    info.size = len(data)
    archive.addfile(info, io.BytesIO(data))


class SyntheticBuild(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.built = self.root / "build"
        self.source = self.built / "wine"
        self.changed = list(build.SOURCES)
        for name in self.changed:
            path = self.source / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(("consumed patched " + name + "\n").encode())
        (self.built / "build.sh").write_bytes(b"export XDG_CACHE_HOME=/work/cache\n")
        with tarfile.open(self.built / "wine.tar.gz", "w:gz") as archive:
            for name in self.changed + ["COPYING.LIB", "unmodified.c"]:
                add(archive, "wine-" + build.WINE + "/" + name, ("pristine " + name + "\n").encode())
        patches = []
        for name, _, source in build.PATCHES:
            (self.built / name).write_bytes(("fixture " + name + "\n").encode())
            patches.append((name, build.sha(self.built / name), source))
        (self.built / "build.py").write_bytes(b"fixture builder\n")
        self.addCleanup(patch.stopall)
        patch.object(build, "ARCHIVE_SHA", build.sha(self.built / "wine.tar.gz")).start()
        sources = {name: ("ab" * 20, build.git_blob(self.source / name)) for name in self.changed}
        patch.object(build, "SOURCES", sources).start()
        patch.object(build, "PATCHES", patches).start()
        artifacts = {}
        for name, (_, _, machine) in package.FILES.items():
            path = self.built / "out" / name
            path.parent.mkdir(parents=True)
            path.write_bytes(pe_image(machine))
            artifacts[name] = {"sha256": build.sha(path), "size": path.stat().st_size,
                               "wine_builtin_marker": True}
        self.record = {"wine_commit": build.WINE, "sdk_image": build.SDK,
                       "wine_archive_sha256": build.ARCHIVE_SHA,
                       "patches": [dict(file=n, sha256=d, source=s) for n, d, s in patches],
                       "patched_sources": {name: {"sha256": build.sha(self.source / name),
                                                  "git_blob_before": before, "git_blob_after": after}
                                           for name, (before, after) in sources.items()},
                       "reader_checkpoint": {"after": build.READER_PATCH, "path": build.TABLE,
                                             "git_blob": build.READER_BLOB},
                       "build_script_sha256": build.sha(self.built / "build.sh"),
                       "builder_sha256": build.sha(self.built / "build.py"),
                       "artifacts": artifacts}
        (self.built / "build-manifest.json").write_text(json.dumps(self.record))
        self.ge = self.ge_archive({"files/lib/wine/i386-windows/kernel32.dll": b"k32",
                                   "files/lib/wine/x86_64-windows/kernel32.dll": b"k64"})

    def ge_archive(self, extra, preimage=b"original msi"):
        path = self.root / ("ge-%d.tar.gz" % len(list(self.root.glob("ge-*"))))
        with tarfile.open(path, "w:gz") as archive:
            for target, _, _ in package.FILES.values():
                add(archive, package.GE_ROOT + "/" + target, preimage)
            for name, data in extra.items():
                add(archive, package.GE_ROOT + "/" + name, data)
        files = {name: (target, hashlib.sha256(b"original msi").hexdigest(), machine)
                 for name, (target, _, machine) in package.FILES.items()}
        patch.object(package, "FILES", files).start()
        patch.object(package, "GE_SHA", build.sha(path)).start()
        return path


class CorrespondingSourceTests(SyntheticBuild):
    def test_bundle_preserves_consumed_change_and_pristine_remainder(self):
        output = self.root / "source.tar.gz"
        package.source_archive(output, self.built, self.record)
        with tarfile.open(output) as archive:
            for name in self.changed:
                self.assertEqual(archive.extractfile("wine/" + name).read(),
                                 (self.source / name).read_bytes())
            self.assertEqual(archive.extractfile("wine/unmodified.c").read(),
                             b"pristine unmodified.c\n")
            self.assertEqual(archive.extractfile("build.sh").read(),
                             (self.built / "build.sh").read_bytes())
            self.assertEqual(archive.extractfile("SDK.txt").read(), (build.SDK + "\n").encode())

    def test_changed_consumed_source_or_recipe_is_rejected_before_archive_creation(self):
        for path, error in [(self.source / name, "consumed source changed") for name in self.changed] + [
                (self.built / "build.sh", "consumed build recipe changed")]:
            with self.subTest(path=path.name):
                original = path.read_bytes()
                path.write_bytes(original + b"later edit\n")
                output = self.root / "not-created.tar.gz"
                with self.assertRaisesRegex(ValueError, error):
                    package.source_archive(output, self.built, self.record)
                self.assertFalse(output.exists())
                path.write_bytes(original)

    def test_changed_pristine_archive_is_rejected(self):
        archive = self.built / "wine.tar.gz"
        archive.write_bytes(archive.read_bytes() + b"changed")
        output = self.root / "not-created.tar.gz"
        with self.assertRaisesRegex(ValueError, "source archive changed"):
            package.source_archive(output, self.built, self.record)
        self.assertFalse(output.exists())


class ComponentTests(SyntheticBuild):
    def test_component_binds_both_preimages_and_is_deterministic(self):
        first = package.package(self.built, self.ge, self.root / "one")
        second = package.package(self.built, self.ge, self.root / "two")
        self.assertEqual(first["component"]["sha256"], second["component"]["sha256"])
        files = {row["path"]: row for row in first["manifest"]["files"]}
        self.assertEqual(set(files), {target for target, _, _ in package.FILES.values()})
        for name, (target, original, _) in package.FILES.items():
            self.assertEqual(files[target]["original_sha256"], original)
            self.assertEqual(files[target]["source"], name)
            self.assertEqual(files[target]["corrected_sha256"], self.record["artifacts"][name]["sha256"])
        self.assertEqual(set(first["manifest"]["artifacts"]), set(package.ROLES))
        with tarfile.open(self.root / "one" / "msi-reader.tar.gz") as archive:
            names = archive.getnames()
            self.assertTrue(all(name.startswith("msi-reader/") for name in names))
            self.assertIn("msi-reader/i386-windows/msi.dll", names)
            self.assertIn("msi-reader/wine-source.tar.gz", names)
            self.assertTrue(all(member.mtime == 0 for member in archive.getmembers()))
            self.assertEqual(archive.extractfile("msi-reader/COPYING.LIB").read(),
                             b"pristine COPYING.LIB\n")

    def test_changed_preimage_or_missing_import_refuses_before_output(self):
        for extra, preimage, error in [
                ({"files/lib/wine/i386-windows/kernel32.dll": b"k32",
                  "files/lib/wine/x86_64-windows/kernel32.dll": b"k64"}, b"changed msi",
                 "GE MSI preimage changed"),
                ({"files/lib/wine/x86_64-windows/kernel32.dll": b"k64"}, b"original msi",
                 "imports unavailable")]:
            with self.subTest(error=error):
                ge = self.ge_archive(extra, preimage)
                output = self.root / "refused"
                with self.assertRaisesRegex(ValueError, error):
                    package.package(self.built, ge, output)
                self.assertFalse(output.exists())

    def test_import_reader_requires_the_declared_architecture(self):
        path = self.root / "image.dll"
        path.write_bytes(pe_image(0x014C, ("ntdll.dll", "KERNEL32.dll")))
        self.assertEqual(package.pe_imports(path, 0x014C), ["kernel32.dll", "ntdll.dll"])
        with self.assertRaisesRegex(ValueError, "architecture"):
            package.pe_imports(path, 0x8664)
        path.write_bytes(pe_image(0x8664, builtin=False))
        self.assertFalse(build.check_builtin_pe(path, 0x8664))


if __name__ == "__main__":
    unittest.main()

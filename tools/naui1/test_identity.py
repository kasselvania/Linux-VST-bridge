import os
import pathlib
import tempfile
import unittest
from unittest.mock import patch
import identity
from identity import discover
from common import Refusal, file_identity, opened, read
from fixtures import application, archive


class IdentityTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(); self.addCleanup(self.tmp.cleanup)
        self.drive = pathlib.Path(self.tmp.name).resolve(); self.root = application(self.drive)

    def test_exact_identity_and_no_source_export(self):
        result, _ = discover(self.drive)
        self.assertEqual(result['renderer_family'], 'electron_chromium')
        self.assertEqual(result['application_version'], '3.22.0')
        self.assertEqual(result['architecture'], 'x64')
        self.assertNotIn('require(', str(result)); self.assertNotIn(str(self.drive), str(result))

    def test_each_positive_identity_family_required(self):
        for file in ('icudtl.dat', 'resources.pak', 'v8_context_snapshot.bin'):
            p = self.root/file; b = p.read_bytes(); p.unlink()
            self.assertEqual(discover(self.drive)[0]['renderer_family'], 'unresolved')
            p.write_bytes(b)

    def test_misleading_product_or_missing_main_or_electron_import(self):
        p = self.root/'resources/app.asar'
        p.write_bytes(archive({'name': 'other', 'main': 'main.js'}))
        with self.assertRaises(Refusal): discover(self.drive)
        p.write_bytes(archive({'name': 'native-access', 'main': 'missing.js'}))
        with self.assertRaises(Refusal): discover(self.drive)
        p.write_bytes(archive(main=b'console.log("hello")'))
        self.assertEqual(discover(self.drive)[0]['renderer_family'], 'unresolved')

    def test_pe_product_name_required(self):
        from fixtures import executable
        (self.root/'Native Access.exe').write_bytes(executable('Misleading'))
        with self.assertRaises(Refusal): discover(self.drive)

    def test_duplicate_roots_refuse(self):
        application(self.drive/'duplicate')
        with self.assertRaises(Refusal): discover(self.drive)

    def test_symlink_root_and_file_and_hardlink_refuse(self):
        exe = self.root/'Native Access.exe'; b = exe.read_bytes()
        other = self.drive/'other'; other.write_bytes(b); exe.unlink(); exe.symlink_to(other)
        with self.assertRaises((Refusal, OSError)): discover(self.drive)
        exe.unlink(); os.link(other, exe)
        with self.assertRaises(Refusal): discover(self.drive)
        exe.unlink(); other.unlink(); exe.write_bytes(b)
        alias = self.drive/'alias'; alias.symlink_to(self.drive, target_is_directory=True)
        with self.assertRaises(Refusal): discover(alias)

    def test_optional_symlink_refused(self):
        (self.root/'libEGL.dll').symlink_to(self.root/'icudtl.dat')
        with self.assertRaises((Refusal, OSError)): discover(self.drive)

    def test_replacement_between_hash_and_parse_and_after_parse(self):
        original = identity.pe
        for name in ('Native Access.exe', 'resources/app.asar'):
            p = self.root/name; b = p.read_bytes()
            def change(f):
                ans = original(f); p.write_bytes(p.read_bytes()+b'changed'); return ans
            with patch.object(identity, 'pe', side_effect=change):
                with self.assertRaises(Refusal): discover(self.drive)
            p.write_bytes(b)

    def test_digest_drift_pe_resource_asar(self):
        for name in ('Native Access.exe', 'resources/app.asar', 'icudtl.dat'):
            p = self.root/name; b = p.read_bytes(); expected = file_identity(p)['sha256']; p.write_bytes(b+b'x')
            with self.assertRaises(Refusal): read(p, expected=expected)
            p.write_bytes(b)

    def test_asar_bounds_duplicate_keys_truncation_and_links(self):
        p = self.root/'resources/app.asar'; original = p.read_bytes()
        for data in (b'', original[:10], b'\xff'*16, original[:20]+b'\0'*16):
            p.write_bytes(data)
            with self.assertRaises((Refusal, ValueError, OSError)): discover(self.drive)
        p.write_bytes(original)
        # Selected package/main metadata are bounded; invalid main is never extracted.
        for main in ('../outside.js', '/outside.js', 'missing.js'):
            p.write_bytes(archive({'name': 'native-access', 'main': main}))
            with self.assertRaises(Refusal): discover(self.drive)

    def test_nonregular_permissions_and_bound(self):
        p = self.root/'icudtl.dat'; p.chmod(0o666)
        with self.assertRaises(Refusal): opened(p)
        p.chmod(0o600)
        with self.assertRaises(Refusal): file_identity(p, maximum=1)

    def test_missing_application(self):
        (self.root/'Native Access.exe').unlink()
        with self.assertRaises(Refusal): discover(self.drive)


if __name__ == '__main__': unittest.main()

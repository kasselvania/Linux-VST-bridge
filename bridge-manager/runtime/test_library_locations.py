"""Closed location plans: first-party directories only, no Wine or vendor inputs."""
import copy
import stat
from types import SimpleNamespace
import os
import pathlib
import tempfile
import unittest
from unittest.mock import patch

import session


class LocationPlanTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = pathlib.Path(self.temp.name).resolve()
        self.folder = self.root / 'Samples'
        self.folder.mkdir()
        (self.folder / 'first-party.txt').write_text('preserve me')
        self.mapping = self.root / 'setup/compatdata/pfx/dosdevices/l:'
        self.mapping.parent.mkdir(parents=True)
        self.mapping.symlink_to(self.folder)
        stat = self.folder.stat()
        self.reg = {'environment': {'root': str(self.root / 'setup'), 'library_locations': 'a' * 64}}
        self.plan = {'schema': 1, 'digest': 'a' * 64, 'locations': [{
            'path': str(self.folder), 'drive': 'L:', 'read_only': False, 'available': True,
            'device': stat.st_dev, 'inode': stat.st_ino, 'volume': 'fixture-volume',
            'handle_type': 1, 'handle': [1, 2, 3]}]}

    def apply(self, plan=None):
        return session.library_location_environment(self.reg, self.plan if plan is None else plan, {})

    def test_unconfigured_bytes_need_no_plan_but_even_empty_configured_set_requires_it(self):
        inherited = {'DISPLAY': ':1'}
        self.assertIs(session.library_location_environment({'environment': {}}, None, inherited), inherited)
        with self.assertRaisesRegex(RuntimeError, 'launch_plan_missing'):
            session.library_location_environment(self.reg, None, inherited)
        empty = {**self.plan, 'locations': []}
        env = self.apply(empty)
        self.assertEqual(env, {'LVB_LIBRARY_LOCATIONS_CONSUMED': 'a' * 64})

    def test_read_only_grants_canonical_original_path_and_transport_preserves_it(self):
        self.plan['locations'][0]['read_only'] = True
        with patch.object(session, 'library_location_identity') as check:
            env = self.apply()
        check.assert_called_once()
        self.assertEqual(env['PRESSURE_VESSEL_FILESYSTEMS_RO'], str(self.folder))
        self.assertNotIn('PRESSURE_VESSEL_FILESYSTEMS_RW', env)
        with patch.object(session, 'validate_runtime', return_value=self.root / 'runtime'):
            session.transport_environment({'shared_runtime': True}, env)
        self.assertEqual(env['PRESSURE_VESSEL_FILESYSTEMS_RO'], str(self.folder))
        self.assertEqual(env['PRESSURE_VESSEL_FILESYSTEMS_RW'], str(self.root / 'runtime'))

    def test_read_write_default_and_transport_both_reach_pressure_vessel(self):
        with patch.object(session, 'library_location_identity'):
            env = self.apply()
        with patch.object(session, 'validate_runtime', return_value=self.root / 'runtime'):
            session.transport_environment({'shared_runtime': True}, env)
        self.assertEqual(env['PRESSURE_VESSEL_FILESYSTEMS_RW'], str(self.folder) + ':' + str(self.root / 'runtime'))
        self.assertEqual((self.folder / 'first-party.txt').read_text(), 'preserve me')

    def test_replaced_directory_and_stale_digest_refuse(self):
        wrong = copy.deepcopy(self.plan)
        wrong['digest'] = 'b' * 64
        with self.assertRaisesRegex(RuntimeError, 'launch_plan_changed'):
            self.apply(wrong)
        self.folder.rename(self.root / 'retained')
        self.folder.mkdir()
        with self.assertRaisesRegex(RuntimeError, 'directory_changed'):
            self.apply()
        self.assertEqual((self.root / 'retained/first-party.txt').read_text(), 'preserve me')

    def test_foreign_letter_target_is_not_repaired_or_followed(self):
        self.mapping.unlink()
        foreign = self.root / 'foreign'
        foreign.mkdir()
        self.mapping.symlink_to(foreign)
        with self.assertRaisesRegex(RuntimeError, 'mapping_changed'):
            self.apply()
        self.assertEqual(os.readlink(self.mapping), str(foreign))

    def test_unavailable_cannot_expose_a_substituted_mount_or_empty_mountpoint(self):
        self.plan['locations'][0]['available'] = False
        with self.assertRaisesRegex(RuntimeError, 'library_location_unavailable'):
            self.apply()
        self.mapping.unlink()
        with self.assertRaisesRegex(RuntimeError, 'library_location_unavailable'):
            self.apply()
        self.assertTrue(self.folder.is_dir())

    def test_closed_grant_refuses_reserved_letter_and_unknown_inputs(self):
        for drive in ('C:', 'Z:', 'S:'):
            changed = copy.deepcopy(self.plan)
            changed['locations'][0]['drive'] = drive
            with self.assertRaisesRegex(RuntimeError, 'grant_invalid'):
                self.apply(changed)
        changed = copy.deepcopy(self.plan)
        changed['locations'][0]['shell'] = 'unused'
        with self.assertRaisesRegex(RuntimeError, 'grant_invalid'):
            self.apply(changed)

    def test_btrfs_virtual_device_resolves_through_exact_handle_mount_id(self):
        grant = self.plan['locations'][0]
        calls = []
        class Kernel:
            def name_to_handle_at(inner, fd, name, out, mount, flags):
                calls.append((name, flags))
                out._obj.size = 3
                out._obj.kind = 1
                out._obj.bytes[:3] = (1, 2, 3)
                mount._obj.value = 902
                return 0
        original = pathlib.Path.stat
        def metadata(path, *args, **kwargs):
            if str(path) in ('/dev/disk/by-uuid/fixture-volume', '/dev/block fixture'):
                # Deliberately different from directory st_dev (Btrfs subvolume).
                return SimpleNamespace(st_mode=stat.S_IFBLK, st_rdev=123456)
            return original(path, *args, **kwargs)
        mountinfo = '902 1 0:45 / /samples rw - btrfs /dev/block\\040fixture rw\n'
        fd = os.open(self.folder, os.O_RDONLY)
        try:
            with patch.object(session.ctypes, 'CDLL', return_value=Kernel()), \
                    patch.object(pathlib.Path, 'stat', metadata), \
                    patch.object(pathlib.Path, 'read_text', return_value=mountinfo):
                session.library_location_identity(fd, grant)
        finally:
            os.close(fd)
        self.assertEqual(calls, [(b'', 0x1000), (b'', 0x1000)])

    def test_same_directory_number_on_changed_handle_or_volume_refuses(self):
        grant = self.plan['locations'][0]
        class Kernel:
            def name_to_handle_at(inner, fd, name, out, mount, flags):
                out._obj.size = 3
                out._obj.kind = 1
                out._obj.bytes[:3] = (3, 2, 1)
                mount._obj.value = 902
                return 0
        fd = os.open(self.folder, os.O_RDONLY)
        try:
            with patch.object(session.ctypes, 'CDLL', return_value=Kernel()):
                with self.assertRaisesRegex(RuntimeError, 'handle_changed'):
                    session.library_location_identity(fd, grant)
        finally:
            os.close(fd)

    def test_directory_symlink_is_not_an_authorized_original_path(self):
        real = self.root / 'renamed'
        self.folder.rename(real)
        self.folder.symlink_to(real)
        with self.assertRaisesRegex(RuntimeError, 'path_changed'):
            self.apply()


if __name__ == '__main__':
    unittest.main()

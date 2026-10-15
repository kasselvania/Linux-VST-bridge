import copy, json, os, pathlib, sys, tempfile, time, unittest
from unittest.mock import patch
import session as s
import ownership

PROFILE = json.loads((pathlib.Path(__file__).parents[1] / 'src/vendor_profiles/native_access.json').read_text())
DEFAULT = PROFILE['registry_defaults'][0]
GUID = s.REGISTRY_KNOWN_FOLDERS['Downloads']


class FakeReg:
    """Wine reg.exe query/add semantics over an in-memory per-user hive."""
    def __init__(self, values=None, fail=None):
        self.values = {(k.lower(), n.lower()): (k, n, v) for (k, n), v in (values or {}).items()}
        self.calls = []; self.fail = fail or {}

    def __call__(self, args):
        self.calls.append(list(args))
        failure = self.fail.get(args[1])
        if isinstance(failure, BaseException): raise failure
        if failure is not None: return failure, b''
        key, name = args[2], args[4]
        if args[1] == 'add':
            self.values[(key.lower(), name.lower())] = (key, name, args[8])
            return 0, b'The operation completed successfully.\r\n'
        row = self.values.get((key.lower(), name.lower()))
        if row is None: return 1, b''
        text = '\r\nHKEY_CURRENT_USER\\' + row[0][5:] + '\r\n    ' + row[1] + '    REG_SZ    ' + row[2] + '\r\n\r\n'
        return 0, text.encode()

    def adds(self): return [call for call in self.calls if call[1] == 'add']


class RegistryDefaultTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.prefix = pathlib.Path(self.temp.name).resolve() / 'compatdata/pfx'
        self.user = self.prefix / 'drive_c/users/steamuser'
        self.user.mkdir(parents=True)
        (self.prefix / 'drive_c/users/Public').mkdir()

    def tearDown(self): self.temp.cleanup()

    def shell(self, value='C:\\users\\steamuser\\Downloads', current=None):
        values = {(s.REGISTRY_SHELL_FOLDERS, GUID): value}
        if current is not None: values[(DEFAULT['key'], 'DownloadLocation')] = current
        return values

    def test_bundled_profile_is_admitted_and_profiles_without_defaults_stay_valid(self):
        self.assertEqual(s.registry_defaults_admitted(PROFILE), [DEFAULT])
        self.assertEqual(DEFAULT['key'], 'HKCU\\Software\\Native Instruments\\Native Access')
        plain = dict(PROFILE); del plain['registry_defaults']
        self.assertEqual(s.registry_defaults_admitted(plain), [])

    def test_admission_refuses_other_hives_tokens_types_extents_and_fields(self):
        def changed(**fields): return dict(DEFAULT, **fields)
        for case, defaults in [
            ('hklm', [changed(key='HKLM\\Software\\Vendor')]),
            ('hive spelled out', [changed(key='HKEY_CURRENT_USER\\Software')]),
            ('empty part', [changed(key='HKCU\\Software\\\\Vendor')]),
            ('overlong part', [changed(key='HKCU\\' + 'A' * 65)]),
            ('too deep', [changed(key='HKCU' + '\\A' * 9)]),
            ('expansion', [changed(key='HKCU\\Software\\%USERPROFILE%')]),
            ('name charset', [changed(name='Value;x')]),
            ('overlong name', [changed(name='N' * 65)]),
            ('unknown folder', [changed(value={'known_folder': 'Desktop'})]),
            ('free string', [changed(value='C:\\users\\steamuser\\Downloads')]),
            ('extra value field', [changed(value={'known_folder': 'Downloads', 'suffix': 'x'})]),
            ('expand type', [changed(type='REG_EXPAND_SZ')]),
            ('extra field', [changed(command='reg add')]),
            ('duplicate', [DEFAULT, changed(key=DEFAULT['key'].upper())]),
            ('count', [changed(name='V%d' % i) for i in range(5)]),
            ('not a list', DEFAULT),
        ]:
            with self.subTest(case=case):
                with self.assertRaisesRegex(ValueError, 'managed_application_profile_registry_defaults'):
                    s.registry_defaults_admitted(dict(PROFILE, registry_defaults=defaults))

    def test_absent_value_is_set_to_the_windows_downloads_folder_inside_the_prefix(self):
        reg = FakeReg(self.shell())
        self.assertEqual(s.registry_default_apply(DEFAULT, self.prefix, reg),
            {'known_folder': 'Downloads', 'result': 'applied', 'reason': None})
        self.assertEqual(reg.adds(), [['reg.exe', 'add', DEFAULT['key'], '/v', 'DownloadLocation',
            '/t', 'REG_SZ', '/d', 'C:\\users\\steamuser\\Downloads', '/f']])
        self.assertTrue((self.user / 'Downloads').is_dir())

    def test_empty_value_is_filled_and_an_existing_folder_is_kept(self):
        (self.user / 'Downloads').mkdir(); (self.user / 'Downloads/kept').write_text('x')
        for empty in ('', '   ', '(value not set)'):
            with self.subTest(empty=empty):
                reg = FakeReg(self.shell(current=empty))
                self.assertEqual(s.registry_default_apply(DEFAULT, self.prefix, reg)['result'], 'applied')
                self.assertEqual(reg.values[(DEFAULT['key'].lower(), 'downloadlocation')][2], 'C:\\users\\steamuser\\Downloads')
        self.assertEqual((self.user / 'Downloads/kept').read_text(), 'x')

    def test_windows_user_comes_from_the_prefix_known_folder_record(self):
        (self.prefix / 'drive_c/users/alice').mkdir()
        reg = FakeReg(self.shell('C:\\users\\alice\\Downloads'))
        self.assertEqual(s.registry_default_apply(DEFAULT, self.prefix, reg)['result'], 'applied')
        self.assertEqual(reg.adds()[0][8], 'C:\\users\\alice\\Downloads')
        self.assertTrue((self.prefix / 'drive_c/users/alice/Downloads').is_dir())
        self.assertFalse((self.user / 'Downloads').exists())

    def test_musician_choice_is_never_overwritten_or_reported(self):
        chosen = 'D:\\Sample Libraries\\Private Name'
        reg = FakeReg(self.shell(current=chosen))
        result = s.registry_default_apply(DEFAULT, self.prefix, reg)
        self.assertEqual(result, {'known_folder': 'Downloads', 'result': 'already_set', 'reason': None})
        self.assertEqual(reg.adds(), [])
        self.assertEqual(len(reg.calls), 1)
        self.assertNotIn('Private', json.dumps(result))
        self.assertFalse((self.user / 'Downloads').exists())

    def test_failures_are_reported_and_never_raise(self):
        outside = FakeReg(self.shell('Z:\\home\\someone\\Downloads'))
        cases = [
            ('timeout', FakeReg(fail={'query': TimeoutError()}), 'timeout'),
            ('runner', FakeReg(fail={'query': OSError('runner gone')}), 'runner_unavailable'),
            ('query', FakeReg(fail={'query': 5}), 'query_failed'),
            ('write', FakeReg(self.shell(), fail={'add': 1}), 'write_failed'),
            ('write timeout', FakeReg(self.shell(), fail={'add': TimeoutError()}), 'timeout'),
            ('known folder missing', FakeReg(), 'known_folder_unavailable'),
            ('outside users', outside, 'known_folder_unavailable'),
            ('traversal', FakeReg(self.shell('C:\\users\\..\\Downloads')), 'known_folder_unavailable'),
            ('user missing', FakeReg(self.shell('C:\\users\\bob\\Downloads')), 'windows_user_unavailable'),
        ]
        for case, reg, reason in cases:
            with self.subTest(case=case):
                self.assertEqual(s.registry_default_apply(DEFAULT, self.prefix, reg),
                    {'known_folder': 'Downloads', 'result': 'failed', 'reason': reason})
                if case != 'write' and case != 'write timeout': self.assertEqual(reg.adds(), [])
        unconfirmed = FakeReg(self.shell())
        unconfirmed_add = unconfirmed.__call__
        def lost(args):
            code, out = unconfirmed_add(args)
            if args[1] == 'add': unconfirmed.values.pop((DEFAULT['key'].lower(), 'downloadlocation'))
            return code, out
        self.assertEqual(s.registry_default_apply(DEFAULT, self.prefix, lost)['reason'], 'write_unconfirmed')

    def test_folder_that_cannot_be_a_directory_is_not_written(self):
        (self.user / 'Downloads').write_text('not a folder')
        reg = FakeReg(self.shell())
        self.assertEqual(s.registry_default_apply(DEFAULT, self.prefix, reg)['reason'], 'known_folder_unavailable')
        self.assertEqual(reg.adds(), [])

    def test_owner_reports_failure_without_raising_when_the_runner_is_unavailable(self):
        owner = object.__new__(s.Nad1Owner)
        owner.spec = {'application': {'environment': {'root': str(self.prefix.parents[1])}}}
        owner.root = self.prefix.parents[1]; owner.prefix = self.prefix
        with patch.object(s, 'environment', return_value={'PATH': '/usr/bin'}), \
             patch.object(s.Nad1Owner, 'builtin', side_effect=TimeoutError()):
            self.assertEqual(owner.registry_defaults([DEFAULT]),
                [{'known_folder': 'Downloads', 'result': 'failed', 'reason': 'timeout'}])
        with patch.object(s, 'environment', side_effect=RuntimeError('graphical user session is unavailable')):
            self.assertEqual(owner.registry_defaults([DEFAULT]),
                [{'known_folder': 'Downloads', 'result': 'failed', 'reason': 'runner_unavailable'}])
        self.assertIsNone(owner.registry_defaults([]))


@unittest.skipUnless(sys.platform == 'linux', 'Linux process custody')
class BuiltinCommandTests(unittest.TestCase):
    def owner(self, script):
        from test_installer import Scope
        owner = object.__new__(s.Nad1Owner)
        temp = tempfile.TemporaryDirectory(); self.addCleanup(temp.cleanup)
        owner.root = pathlib.Path(temp.name); (owner.root / 'home').mkdir()
        owner.ledger = ownership.InstallerLedger(Scope(), lambda _: None)
        owner.cancelled = lambda: False
        owner.runtime = type('Runtime', (), {'drain': lambda *_: None,
            'builtin_argv': lambda _, args, cwd: [sys.executable, '-c', script]})()
        return owner

    def test_builtin_output_stays_in_memory_and_exit_is_observed(self):
        owner = self.owner('import sys; sys.stdout.write("    DownloadLocation    REG_SZ    \\r\\n"); sys.exit(1)')
        code, out = owner.builtin(['reg.exe', 'query', DEFAULT['key'], '/v', 'DownloadLocation'], dict(os.environ), time.monotonic() + 10)
        self.assertEqual((code, out), (1, b'    DownloadLocation    REG_SZ    \r\n'))
        self.assertEqual(list(owner.root.iterdir()), [owner.root / 'home'])

    def test_builtin_timeout_is_bounded_and_retires_the_command(self):
        owner = self.owner('import time; time.sleep(30)')
        started = time.monotonic()
        with patch.object(s, 'REGISTRY_DEFAULT_CALL_SECONDS', 0.3):
            with self.assertRaises(TimeoutError):
                owner.builtin(['reg.exe', 'query', DEFAULT['key'], '/v', 'DownloadLocation'], dict(os.environ), time.monotonic() + 10)
        self.assertLess(time.monotonic() - started, 5)
        self.assertEqual(owner.ledger.scope.members(), [])


if __name__ == '__main__':
    unittest.main()

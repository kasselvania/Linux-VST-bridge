import copy, json, os, pathlib, signal, struct, sys, tempfile, time, unittest
from unittest.mock import patch
import session as s
import ownership

PROFILE = json.loads((pathlib.Path(__file__).parents[1] / 'src/vendor_profiles/native_access.json').read_text())
DEFAULT = PROFILE['registry_defaults'][0]
GUID = s.REGISTRY_KNOWN_FOLDERS['Downloads']
TOOL = s.REGISTRY_TOOL
NOT_FOUND = b'reg: Unable to find the specified registry value.\r\n'


class FakeReg:
    """Wine reg.exe query/add semantics over an in-memory per-user hive.

    ``script`` replaces the response of a given call index, to model runner,
    Proton or Wine failures that share reg.exe's exit codes."""
    def __init__(self, values=None, script=None, not_found_on_stdout=False):
        self.values = {(k.lower(), n.lower()): (k, n, t, v) for (k, n), (t, v) in (values or {}).items()}
        self.calls = []; self.script = script or {}; self.not_found_on_stdout = not_found_on_stdout

    def __call__(self, args):
        index = len(self.calls); self.calls.append(list(args))
        if args[0] != TOOL: raise AssertionError('unexpected tool ' + args[0])
        scripted = self.script.get(index)
        if isinstance(scripted, BaseException): raise scripted
        if scripted is not None: return scripted
        key, name = args[2], args[4]
        if args[1] == 'add':
            self.values[(key.lower(), name.lower())] = (key, name, 'REG_SZ', args[8])
            return 0, b'The operation completed successfully.\r\n', b''
        row = self.values.get((key.lower(), name.lower()))
        if row is None: return (1, NOT_FOUND, b'') if self.not_found_on_stdout else (1, b'', NOT_FOUND)
        data = row[3] if isinstance(row[3], bytes) else row[3].encode()
        return 0, b'\r\nHKEY_CURRENT_USER\\' + row[0][5:].encode() + b'\r\n    ' + row[1].encode() + b'    ' \
            + row[2].encode() + b'    ' + data + b'\r\n\r\n', b''

    def adds(self): return [call for call in self.calls if call[1] == 'add']


class RegistryDefaultTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.root = pathlib.Path(self.temp.name).resolve()
        self.user = self.root / 'compatdata/pfx/drive_c/users/steamuser'
        self.user.mkdir(parents=True)
        (self.user.parent / 'Public').mkdir()

    def values(self, folder='C:\\users\\steamuser\\Downloads', current=None, kind='REG_SZ'):
        values = {} if folder is None else {(s.REGISTRY_SHELL_FOLDERS, GUID): ('REG_SZ', folder)}
        if current is not None: values[(DEFAULT['key'], 'DownloadLocation')] = (kind, current)
        return values

    def apply(self, reg, **kwargs): return s.registry_default_apply(DEFAULT, self.root, reg, **kwargs)

    def failed(self, reason): return {'known_folder': 'Downloads', 'result': 'failed', 'reason': reason}

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
            ('dot part', [changed(key='HKCU\\Software\\..')]),
            ('overlong part', [changed(key='HKCU\\' + 'A' * 65)]),
            ('too deep', [changed(key='HKCU' + '\\A' * 9)]),
            ('expansion', [changed(key='HKCU\\Software\\%USERPROFILE%')]),
            ('name charset', [changed(name='Value;x')]),
            ('name starts with a dash', [changed(name='-Value')]),
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

    def test_absent_value_is_set_to_the_windows_downloads_folder_inside_the_setup(self):
        for on_stdout in (False, True):
            with self.subTest(not_found_on_stdout=on_stdout):
                reg = FakeReg(self.values(), not_found_on_stdout=on_stdout)
                self.assertEqual(self.apply(reg), {'known_folder': 'Downloads', 'result': 'applied', 'reason': None})
                self.assertEqual(reg.adds(), [[TOOL, 'add', DEFAULT['key'], '/v', 'DownloadLocation',
                    '/t', 'REG_SZ', '/d', 'C:\\users\\steamuser\\Downloads', '/f']])
                self.assertTrue((self.user / 'Downloads').is_dir())

    def test_value_is_read_twice_as_absent_before_the_write(self):
        reg = FakeReg(self.values())
        self.apply(reg)
        self.assertEqual([(call[1], call[2]) for call in reg.calls], [('query', DEFAULT['key']),
            ('query', s.REGISTRY_SHELL_FOLDERS), ('query', DEFAULT['key']), ('add', DEFAULT['key']), ('query', DEFAULT['key'])])

    def test_empty_value_is_filled_and_an_existing_folder_is_kept(self):
        (self.user / 'Downloads').mkdir(); (self.user / 'Downloads/kept').write_text('x')
        for empty in ('', '   ', '(value not set)'):
            with self.subTest(empty=empty):
                reg = FakeReg(self.values(current=empty))
                self.assertEqual(self.apply(reg)['result'], 'applied')
                self.assertEqual(reg.values[(DEFAULT['key'].lower(), 'downloadlocation')][3], 'C:\\users\\steamuser\\Downloads')
        self.assertEqual((self.user / 'Downloads/kept').read_text(), 'x')

    def test_windows_user_comes_from_the_prefix_known_folder_record(self):
        (self.user.parent / 'alice').mkdir()
        reg = FakeReg(self.values('C:\\users\\alice\\Downloads'))
        self.assertEqual(self.apply(reg)['result'], 'applied')
        self.assertEqual(reg.adds()[0][8], 'C:\\users\\alice\\Downloads')
        self.assertTrue((self.user.parent / 'alice/Downloads').is_dir())
        self.assertFalse((self.user / 'Downloads').exists())

    def test_musician_choice_is_never_overwritten_or_reported(self):
        for kind, chosen in (('REG_SZ', 'D:\\Sample Libraries\\Private Name'), ('REG_EXPAND_SZ', '%USERPROFILE%\\Private'),
                             ('REG_SZ', 'D:\\Bibliothèque Privée')):
            with self.subTest(kind=kind, chosen=chosen):
                reg = FakeReg(self.values(current=chosen, kind=kind))
                result = self.apply(reg)
                self.assertEqual(result, {'known_folder': 'Downloads', 'result': 'already_set', 'reason': None})
                self.assertEqual(reg.adds(), [])
                self.assertEqual(len(reg.calls), 1)
                self.assertNotIn('Priv', json.dumps(result))
        self.assertFalse((self.user / 'Downloads').exists())

    def test_unclear_readings_never_lead_to_a_write(self):
        traceback = b'Traceback (most recent call last):\n  File "proton", line 1\nOSError\n'
        row = b'    DownloadLocation    REG_SZ    D:\\Chosen\r\n'
        for case, script in [
            ('exit 1 without reg.exe not-found text', {0: (1, b'', traceback)}),
            ('exit 1 with a value line', {0: (1, row, NOT_FOUND)}),
            ('exit 0 with no value line', {0: (0, b'\r\nHKEY_CURRENT_USER\\Software\r\n\r\n', b'')}),
            ('other exit code', {0: (5, b'', b'')}),
            ('two value lines', {0: (0, row + row, b'')}),
            ('transient failure on the second reading', {2: (1, b'', b'wine: could not load kernel32.dll\n')}),
            ('transient timeout on the second reading', {2: TimeoutError()}),
            ('known folder query without not-found text', {1: (1, b'', b'')}),
        ]:
            with self.subTest(case=case):
                reg = FakeReg(self.values(), script=script)
                self.assertEqual(self.apply(reg)['result'], 'failed')
                self.assertEqual(reg.adds(), [])
        reg = FakeReg(self.values(), script={0: (1, b'', traceback)})
        self.assertEqual(self.apply(reg), self.failed('query_unreadable'))

    def test_value_set_between_the_two_readings_is_kept(self):
        reg = FakeReg(self.values(), script={2: (0, b'    DownloadLocation    REG_SZ    D:\\Chosen\r\n', b'')})
        self.assertEqual(self.apply(reg)['result'], 'already_set')
        self.assertEqual(reg.adds(), [])

    def test_known_folder_must_be_plain_ascii_inside_the_prefix_user(self):
        for case, folder, reason in [
            ('replacement character', 'C:\\users\\steamuser\\Down\ufffdloads'.encode(), 'query_unreadable'),
            ('latin-1 byte', b'C:\\users\\st\xe9amuser\\Downloads', 'query_unreadable'),
            ('trailing dot', 'C:\\users\\steamuser\\Downloads.', 'known_folder_unavailable'),
            ('variable', 'C:\\users\\steamuser\\%DL%', 'known_folder_unavailable'),
            ('traversal', 'C:\\users\\..\\Downloads', 'known_folder_unavailable'),
            ('outside users', 'Z:\\home\\someone\\Downloads', 'known_folder_unavailable'),
            ('deeper', 'C:\\users\\steamuser\\a\\Downloads', 'known_folder_unavailable'),
            ('user missing', 'C:\\users\\bob\\Downloads', 'windows_user_unavailable'),
            ('record missing', None, 'known_folder_unavailable'),
        ]:
            with self.subTest(case=case):
                reg = FakeReg(self.values(folder))
                self.assertEqual(self.apply(reg), self.failed(reason))
                self.assertEqual(reg.adds(), [])
        self.assertFalse((self.user / 'Downloads').exists())

    def test_symlinked_downloads_must_stay_inside_the_setup(self):
        outside = tempfile.TemporaryDirectory(); self.addCleanup(outside.cleanup)
        (self.user / 'Downloads').symlink_to(outside.name)
        reg = FakeReg(self.values())
        self.assertEqual(self.apply(reg), self.failed('download_folder_outside_setup'))
        self.assertEqual(reg.adds(), [])
        (self.user / 'Downloads').unlink()
        inside = self.root / 'home/Downloads'; inside.mkdir(parents=True)
        (self.user / 'Downloads').symlink_to(inside)
        self.assertEqual(self.apply(FakeReg(self.values()))['result'], 'applied')

    def test_failures_are_reported_and_never_raise(self):
        for case, reg, reason in [
            ('timeout', FakeReg(script={0: TimeoutError()}), 'timeout'),
            ('runner', FakeReg(script={0: OSError('runner gone')}), 'runner_unavailable'),
            ('cancelled', FakeReg(script={0: s.RegistryDefaultFailure('cancelled')}), 'cancelled'),
            ('write', FakeReg(self.values(), script={3: (1, b'', b'')}), 'write_failed'),
            ('write timeout', FakeReg(self.values(), script={3: TimeoutError()}), 'timeout'),
        ]:
            with self.subTest(case=case):
                self.assertEqual(self.apply(reg), self.failed(reason))
        (self.user / 'Downloads').rmdir(); (self.user / 'Downloads').write_text('not a folder')
        self.assertEqual(self.apply(FakeReg(self.values())), self.failed('known_folder_unavailable'))
        (self.user / 'Downloads').unlink()
        unconfirmed = FakeReg(self.values())
        def lost(args):
            result = unconfirmed(args)
            if args[1] == 'add': unconfirmed.values.pop((DEFAULT['key'].lower(), 'downloadlocation'))
            return result
        self.assertEqual(self.apply(lost), self.failed('write_unconfirmed'))

    def test_only_reg_exe_not_found_readings_are_expected_exits(self):
        expected = []
        self.apply(FakeReg(self.values()), absent=lambda: expected.append(1))
        self.assertEqual(len(expected), 2)  # the value, read twice before the write
        expected.clear()
        self.apply(FakeReg(self.values(), script={0: (1, b'', b'Traceback\n')}), absent=lambda: expected.append(1))
        self.assertEqual(expected, [])

    def test_builtin_argv_refuses_any_other_command(self):
        runtime = object.__new__(s.Nad1Runtime)
        for args in (['reg.exe', 'query', DEFAULT['key']], [TOOL, 'delete', DEFAULT['key'], '/f'],
                     [TOOL, 'import', 'x.reg'], ['C:\\windows\\system32\\cmd.exe', '/c', 'reg', 'add'], []):
            with self.subTest(args=args):
                with self.assertRaisesRegex(ValueError, 'dependency_builtin_command'):
                    runtime.builtin_argv(args, self.root)


class EnsureTimingTests(unittest.TestCase):
    """The default is settled after the first service query and before any
    service install or start, so the service reads it when it starts."""
    def setUp(self):
        tmp = tempfile.TemporaryDirectory(); self.addCleanup(tmp.cleanup)
        self.root = pathlib.Path(tmp.name).resolve(); self.drive = self.root / 'compatdata/pfx/drive_c'
        (self.drive / 'users/steamuser').mkdir(parents=True)
        self.installer = self.drive / 'Setup.exe'; self.daemon = self.drive / 'NTKDaemon.exe'
        self.bytes = bytearray(256); self.bytes[:2] = b'MZ'; struct.pack_into('<I', self.bytes, 60, 64)
        self.bytes[64:68] = b'PE\0\0'; self.bytes[68:70] = b'\x64\x86'; self.installer.write_bytes(self.bytes)
        self.artifact = ownership.image_identity(self.installer)
        spec = {'operation': 'a' * 32, 'report': str(self.root / 'result.json'), 'application': {'environment': {'root': str(self.root)}}}
        fixture = {'installer': 'Setup.exe', 'daemon': 'NTKDaemon.exe', 'installer_sha256': self.artifact['sha256'], 'installer_size': 256}
        self.ledger = type('Ledger', (), {'harvest': lambda _: None, 'phase': 'dependency_query', 'first_failure': None})()
        self.owner = s.Nad1Owner(spec, self.ledger, None, lambda: False, fixture=fixture)
        self.owner.managed_contract = {'fixture': 'managed', 'profile': PROFILE}
        self.events = []; self.registered = False; self.state = 1
        def command(action):
            self.events.append(('service', action))
            if action == 'install': self.daemon.write_bytes(self.bytes); self.registered = True; return None
            if action == 'start': self.state = 4
            return {'registration': 'exact' if self.registered else 'absent', 'state': self.state,
                'image_sha256': self.artifact['sha256'], 'owned_endpoint_mask': 3 if self.state == 4 else 0,
                'windows_pid': 123 if self.state == 4 else 0}
        self.owner.command = command
        self.reg = FakeReg({(s.REGISTRY_SHELL_FOLDERS, GUID): ('REG_SZ', 'C:\\users\\steamuser\\Downloads')})
        def builtin(args, env, deadline):
            self.events.append(('registry', args[1])); self.ledger.phase = 'registry_default'
            self.assertEqual((env['LANG'], env['LC_ALL'], env['WINEDEBUG']), ('C.UTF-8', 'C.UTF-8', '-all'))
            self.assertGreater(deadline, time.monotonic())
            return self.reg(args)
        self.owner.builtin = builtin
        def scan(*_):
            if self.state != 4: return {'candidates': [], 'private': [], 'unavailable': 0}
            return {'candidates': [{'prefix_relation': 'same', 'exact': True}],
                'private': [{'prefix_relation': 'same', 'exact': True, 'linux_pid': 10, 'start_ticks': 20}], 'unavailable': 0}
        for patcher in (patch.object(ownership, 'census', side_effect=scan), patch.object(s, 'nad1_generation_owned', return_value=True),
                        patch.object(s, 'environment', return_value={'PATH': '/usr/bin', 'LANG': 'C.UTF-8'})):
            patcher.start(); self.addCleanup(patcher.stop)

    def outcome(self): return self.owner.value()['registry_defaults']

    def test_default_is_written_before_the_service_is_installed_or_started(self):
        self.owner.ensure(True)
        self.assertTrue(self.owner.ready)
        first_service_change = min(self.events.index(('service', action)) for action in ('install', 'start'))
        writes = [i for i, event in enumerate(self.events) if event == ('registry', 'add')]
        self.assertEqual(len(writes), 1)
        self.assertEqual(self.events[0], ('service', 'query'))
        self.assertLess(writes[0], first_service_change)
        self.assertEqual(self.outcome(), [{'known_folder': 'Downloads', 'result': 'applied', 'reason': None}])
        self.assertEqual(self.ledger.phase, 'dependency_query')

    def test_service_already_running_still_gets_the_default_and_says_so(self):
        self.daemon.write_bytes(self.bytes); self.registered = True; self.state = 4
        self.owner.ensure(True, ownership.image_identity(self.daemon))
        self.assertTrue(self.owner.ready)
        self.assertNotIn(('service', 'start'), self.events)
        self.assertEqual(self.outcome(), [{'known_folder': 'Downloads', 'result': 'applied_after_service_start', 'reason': None}])

    def test_registry_failure_never_blocks_the_service_or_launch_preparation(self):
        self.reg.script = {0: TimeoutError()}
        self.owner.ensure(True)
        self.assertTrue(self.owner.ready)
        self.assertIn(('service', 'start'), self.events)
        self.assertEqual(self.outcome(), [{'known_folder': 'Downloads', 'result': 'failed', 'reason': 'timeout'}])
        self.assertEqual(self.ledger.phase, 'dependency_query')

    def test_profile_without_defaults_runs_no_registry_command(self):
        self.owner.managed_contract = {'fixture': 'managed', 'profile': {k: v for k, v in PROFILE.items() if k != 'registry_defaults'}}
        self.owner.ensure(True)
        self.assertNotIn('registry', {event[0] for event in self.events})
        self.assertIsNone(self.outcome())

    def test_expected_not_found_exit_is_not_the_ledger_first_failure(self):
        committed = []
        ledger = type('Ledger', (), {'commit': lambda _: committed.append(1)})()
        self.owner.ledger = ledger; self.owner.builtin_identity = (10, 20)
        for failure, kept in [({'pid': 10, 'start_ticks': 20, 'status': 1}, False),
                              ({'pid': 10, 'start_ticks': 20, 'status': 2}, True),
                              ({'pid': 11, 'start_ticks': 20, 'status': 1}, True)]:
            with self.subTest(failure=failure):
                ledger.first_failure = dict(failure)
                self.owner._builtin_absent()
                self.assertEqual(ledger.first_failure, failure if kept else None)
        self.assertEqual(len(committed), 1)


FORWARDING_CLIENT = r'''
import os, signal, subprocess, sys, time
remote = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(60)"], start_new_session=True)
open(sys.argv[1], "w").write(str(remote.pid))
def forward(sig, _):
    remote.send_signal(sig); remote.wait(); os._exit(143)
signal.signal(signal.SIGTERM, forward)
time.sleep(60)
'''
DEAF_CLIENT = FORWARDING_CLIENT.replace('signal.signal(signal.SIGTERM, forward)', 'signal.signal(signal.SIGTERM, signal.SIG_IGN)')


@unittest.skipUnless(sys.platform == 'linux', 'Linux process custody')
class BuiltinCommandTests(unittest.TestCase):
    """The launch client relays to a separately owned Windows process, modelled
    here by a child in its own session that only a forwarded signal reaches."""
    def owner(self, script, *args):
        from test_installer import Scope
        owner = object.__new__(s.Nad1Owner)
        temp = tempfile.TemporaryDirectory(); self.addCleanup(temp.cleanup)
        owner.root = pathlib.Path(temp.name); (owner.root / 'home').mkdir()
        owner.ledger = ownership.InstallerLedger(Scope(), lambda _: None)
        owner.cancelled = lambda: False; owner.builtin_identity = None
        owner.runtime = type('Runtime', (), {'drain': lambda *_: None,
            'builtin_argv': lambda _, argv, cwd: [sys.executable, '-c', script, *args]})()
        return owner

    def query(self, owner, deadline=10):
        return owner.builtin([TOOL, 'query', DEFAULT['key'], '/v', 'DownloadLocation'], dict(os.environ), time.monotonic() + deadline)

    def pidfile(self):
        temp = tempfile.TemporaryDirectory(); self.addCleanup(temp.cleanup)
        return pathlib.Path(temp.name) / 'remote'

    def remote(self, path):
        pid = int(path.read_text()); self.addCleanup(lambda: os.kill(pid, signal.SIGKILL) if alive(pid) else None)
        return pid

    def test_builtin_output_stays_in_memory_and_exit_is_observed(self):
        owner = self.owner('import sys; sys.stdout.write("    DownloadLocation    REG_SZ    \\r\\n"); '
                           'sys.stderr.write("reg: Unable to find the specified registry value.\\n"); sys.exit(1)')
        self.assertEqual(self.query(owner), (1, b'    DownloadLocation    REG_SZ    \r\n', b'reg: Unable to find the specified registry value.\n'))
        self.assertEqual(list(owner.root.iterdir()), [owner.root / 'home'])
        self.assertEqual(owner.builtin_identity[0], owner.ledger.first_failure['pid'])

    def test_timeout_forwards_sigterm_through_the_relaying_client(self):
        pidfile = self.pidfile(); owner = self.owner(FORWARDING_CLIENT, str(pidfile))
        started = time.monotonic()
        with patch.object(s, 'REGISTRY_DEFAULT_CALL_SECONDS', 1.0):
            with self.assertRaises(TimeoutError): self.query(owner)
        self.assertLess(time.monotonic() - started, 1.0 + s.REGISTRY_DEFAULT_TERM_SECONDS + 1)
        self.assertFalse(alive(self.remote(pidfile)))
        self.assertEqual(owner.ledger.scope.members(), [])

    def test_client_ignoring_sigterm_is_killed_and_the_remote_waits_for_session_cleanup(self):
        pidfile = self.pidfile(); owner = self.owner(DEAF_CLIENT, str(pidfile))
        with patch.object(s, 'REGISTRY_DEFAULT_CALL_SECONDS', 1.0), patch.object(s, 'REGISTRY_DEFAULT_TERM_SECONDS', 0.5):
            with self.assertRaises(TimeoutError): self.query(owner)
        # Limit, stated: the client is gone, but a launcher-owned process that
        # ignores the forwarded signal is reached only by the operation cgroup.
        self.assertTrue(alive(self.remote(pidfile)))

    def test_no_command_starts_after_the_deadline(self):
        owner = self.owner('raise SystemExit(0)')
        with patch.object(s.subprocess, 'Popen', side_effect=AssertionError('started after deadline')):
            with self.assertRaises(TimeoutError): self.query(owner, deadline=-1)


def alive(pid):
    try: os.kill(pid, 0)
    except ProcessLookupError: return False
    try:
        return pathlib.Path(f'/proc/{pid}/stat').read_text().split(') ')[1].split()[0] != 'Z'
    except OSError: return False


if __name__ == '__main__':
    unittest.main()

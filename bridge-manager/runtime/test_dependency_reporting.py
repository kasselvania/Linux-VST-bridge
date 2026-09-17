"""Installer child result and runner retirement are independent authorities.

Only source-owned local Python processes run here; no Wine or vendor installer.
The adapter frames model the existing Windows handle-result protocol.
"""
import json
import pathlib
import subprocess
import sys
import tempfile
import time
import types
import unittest
from unittest.mock import patch
import session as s


class ReportingTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = pathlib.Path(self.tmp.name)
        self.owner = types.SimpleNamespace(directory=self.root, op='a'*32, token='b'*64,
                                          installer_sha='c'*64, installer_size=256)
        self.stage = {}
        self.observer = s.Nad1InstallObservation(self.owner, self.stage)

    def root_frame(self):
        o = self.owner
        return f'NAD1_INSTALL_ROOT_V1 {o.op} {o.token} {o.installer_sha} 256 123 456\n'.encode()

    def exit_frame(self, status=100):
        return f'NAD1_INSTALL_V1 {self.owner.op} {self.owner.token} {status}\n'.encode()

    def test_root_is_retained_before_exit_and_partial_frames_wait(self):
        raw = self.root_frame()
        self.observer.feed(raw[:-1])
        self.assertNotIn('root_binding', self.stage)
        self.observer.feed(raw[-1:])
        self.assertEqual(self.stage['root_binding']['status'], 'bound')
        self.assertTrue((self.root / (self.owner.op+'-installer-root.private.json')).exists())
        for byte in self.exit_frame():
            self.observer.feed(bytes([byte]))
        self.assertEqual(self.stage['installer_result']['installer_exit'], 100)
        self.assertEqual(self.observer.failure(), 'dependency_installer_nonzero')
        self.assertNotIn('123', json.dumps(self.stage))
        self.assertNotIn(self.owner.token, json.dumps(self.stage))

    def test_exit_without_root_or_changed_identity_refuses(self):
        with self.assertRaises(ValueError):
            self.observer.feed(self.exit_frame())
        for old, new in [(self.owner.op, 'd'*32), (self.owner.token, 'd'*64),
                         (self.owner.installer_sha, 'd'*64), ('256', '257'),
                         ('123', '0'), ('456', '0')]:
            obs = s.Nad1InstallObservation(self.owner, {})
            with self.subTest(old=old), self.assertRaises(ValueError):
                obs.feed(self.root_frame().replace(old.encode(), new.encode()))
        self.assertEqual(list(self.root.iterdir()), [])

    def test_duplicate_and_out_of_range_results_refuse(self):
        self.observer.feed(self.root_frame())
        for value in ['-1', '4294967296', '1.0', 'unknown']:
            with self.subTest(value=value), self.assertRaises(ValueError):
                self.observer.feed(self.exit_frame(value))
        self.observer.feed(self.exit_frame())
        before = (self.root/(self.owner.op+'-installer-result.private.json')).read_bytes()
        with self.assertRaises(ValueError):
            self.observer.feed(self.exit_frame(0))
        self.assertEqual(before, (self.root/(self.owner.op+'-installer-result.private.json')).read_bytes())

    def test_changed_result_identity_and_duplicate_root_refuse(self):
        self.observer.feed(self.root_frame())
        with self.assertRaises(ValueError):
            self.observer.feed(self.exit_frame().replace(self.owner.token.encode(), b'd'*64))
        self.assertIsNone(self.observer.result)
        with self.assertRaises(ValueError):
            self.observer.feed(self.root_frame())

    def test_old_adapter_144_is_not_falsely_claimed_as_child_exit(self):
        self.observer.feed(self.root_frame()+self.exit_frame(144))
        self.assertIsNone(self.stage['installer_result']['installer_exit'])
        self.assertEqual(self.observer.failure(), 'dependency_installer_wait_or_exit_unavailable')

    def test_unterminated_or_oversized_output_cannot_be_completion(self):
        self.observer.feed(self.root_frame()+self.exit_frame()[:-1])
        self.assertIsNone(self.observer.result)
        with self.assertRaisesRegex(ValueError, 'extent'):
            self.observer.feed(b'x'*65537)

    def command_fixture(self, status, linger, clock=None, stderr_frames=False, emit_result=True, runner_status=None):
        env = {'root': str(self.root), 'runner': {'entry_point': '/unused', 'proton': '/unused'}}
        (self.root/'home').mkdir()
        spec = {'operation': self.owner.op, 'report': str(self.root/'result.json'),
                'application': {'environment': env}, 'installer_launch': {'path': '/unused'}}
        class Ledger:
            def launcher(inner, child, phase):
                inner.child = child
            def harvest(inner):
                # This fixture substitutes only the Linux process-custody owner.
                # Production uses InstallerLedger's generation-safe waitid owner.
                inner.child.poll()
        ledger = Ledger()
        owner = s.Nad1Owner(spec, ledger, None, lambda: False, fixture={
            'installer': 'Setup.exe', 'daemon': 'NTKDaemon.exe',
            'installer_sha256': self.owner.installer_sha, 'installer_size': 256})
        owner.runtime = type("Runtime", (), {"argv": lambda *_: ["generated-command"]})()
        owner.token = self.owner.token
        raw = self.root_frame()+(self.exit_frame(status) if emit_result else b'')
        source = ('import os,time,subprocess,sys\n'
                  f'child=subprocess.run([sys.executable,"-c","raise SystemExit({status})"])\n'
                  f'os.write({2 if stderr_frames else 1},{raw!r})\n'
                  f'time.sleep({30 if linger else 0})\n'
                  f'raise SystemExit({runner_status!r} if {runner_status!r} is not None else child.returncode)\n')
        real_popen = subprocess.Popen
        children = []
        def launch(*args, **kwargs):
            child = real_popen([sys.executable, '-c', source], **kwargs)
            children.append(child)
            return child
        def cleanup():
            for child in children:
                if child.poll() is None: child.terminate()
                child.wait(timeout=5)
        self.addCleanup(cleanup)
        with patch.object(s, 'environment', return_value={}), patch.object(s.subprocess, 'Popen', side_effect=launch):
            if clock:
                with patch.object(s.time, 'monotonic', side_effect=clock):
                    try: owner.command('install')
                    except ValueError as e: return owner, children[0], str(e)
            else:
                try: owner.command('install')
                except ValueError as e: return owner, children[0], str(e)
        return owner, children[0], None

    def test_nonzero_child_result_is_reported_while_runner_is_still_live(self):
        start = time.monotonic()
        owner, child, error = self.command_fixture(100, True)
        self.assertLess(time.monotonic()-start, 5)
        self.assertIsNone(child.poll())
        self.assertEqual(error, 'dependency_installer_nonzero')
        stage = owner.stages[0]
        self.assertEqual(stage['installer_result']['installer_exit'], 100)
        self.assertIsNone(stage['exit'])
        self.assertEqual(stage['runner_retirement'], 'not_observed_at_command_return')
        # Later exact cleanup does not rewrite the retained first result.
        child.terminate(); child.wait(timeout=5)
        self.assertEqual(owner.stages[0]['installer_result']['installer_exit'], 100)

    def test_zero_child_with_live_runner_reports_runner_timeout_separately(self):
        real_clock = time.monotonic
        start = real_clock()
        def clock():
            return start + (201 if real_clock()-start > .3 else real_clock()-start)
        owner, child, error = self.command_fixture(0, True, clock)
        self.assertIsNone(child.poll())
        self.assertEqual(error, 'dependency_runner_retirement_timeout')
        self.assertEqual(owner.stages[0]['installer_result']['installer_exit'], 0)

    def test_zero_child_and_retired_runner_remain_only_install_exit_authority(self):
        owner, child, error = self.command_fixture(0, False)
        self.assertIsNone(error)
        self.assertEqual(child.returncode, 0)
        self.assertEqual(owner.stages[0]['result'], 'outer_zero_only')
        self.assertFalse(owner.ready)

    def test_zero_runner_without_installer_result_is_not_success(self):
        owner, child, error = self.command_fixture(0, False, emit_result=False)
        self.assertEqual(child.returncode, 0)
        self.assertEqual(error, 'dependency_install_acknowledgment')
        self.assertNotIn('installer_result', owner.stages[0])

    def test_runner_failure_does_not_replace_installer_zero(self):
        owner, child, error = self.command_fixture(0, False, runner_status=7)
        self.assertEqual(error, 'dependency_runner_nonzero_after_installer_zero')
        self.assertEqual(owner.stages[0]['installer_result']['installer_exit'], 0)
        self.assertEqual(owner.stages[0]['exit'], 7)

    def test_stderr_cannot_forge_adapter_result(self):
        owner, _, error = self.command_fixture(100, False, stderr_frames=True)
        self.assertEqual(error, 'dependency_install_acknowledgment')
        self.assertNotIn('installer_result', owner.stages[0])


if __name__ == '__main__': unittest.main()

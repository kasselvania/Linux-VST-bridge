import hashlib
import pathlib
import tempfile
import unittest
from unittest.mock import patch
import ap8_fixture as fixture


class FixtureTests(unittest.TestCase):
    def test_vendor_stdout_is_bounded_and_host_validation_remains_mandatory(self):
        stream = fixture.StreamState()
        stream.feed('stdout', b'vendor initialization\n{"event":"lifecycle","sequence":1,"state":"ap8_call","operation":"createComponent"}\n')
        stream.feed('stdout', b'x' * 100000)
        stream.feed('stdout', b'\n{"event":"lifecycle","sequence":2,"state":"ap8_result","operation":"createComponent","result":0}\n')
        self.assertEqual(len(stream.records), 2)
        self.assertEqual(len(stream.vendor), 65536)
        self.assertGreater(stream.vendor_bytes, 100000)
        with self.assertRaises(Exception):
            stream.feed('stdout', b'{"event":"lifecycle","sequence":4,"state":"ap8_call"}\n')
        bad = fixture.StreamState()
        with self.assertRaises(Exception):
            bad.feed('stdout', b'{"event": broken\n')

    def test_installed_input_is_copied_and_verified_without_modifying_it(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            parent = root / 'owned';parent.mkdir()
            scanner = root / 'host.exe';scanner.write_bytes(b'host')
            module = root / 'Vendor.vst3';module.write_bytes(b'Windows module')
            before = (module.read_bytes(), module.stat().st_mtime_ns)
            identity = {'launch_critical_manifest_sha256': '1' * 64}
            with patch.object(fixture, 'environment_parent', return_value=parent), patch.object(fixture, 'verify_runtime', return_value=identity):
                env = fixture.create(scanner, module, source_sha256='2' * 64)
                self.assertEqual(env.module.name, module.name)
                self.assertEqual(env.module.read_bytes(), before[0])
                self.assertEqual((module.read_bytes(), module.stat().st_mtime_ns), before)
                fixture.verify_environment(env, runner_identity_sha256='1' * 64)
                env.module.write_bytes(b'wrong module')
                with self.assertRaisesRegex(RuntimeError, 'changed'):
                    fixture.verify_environment(env, runner_identity_sha256='1' * 64)

    def test_reporting_failure_keeps_the_process_outcome_and_containment_separate(self):
        import types
        for complete in (True, False):
            with self.subTest(complete=complete), tempfile.TemporaryDirectory() as directory:
                result = {'raw_exit': 90, 'records': [{'state': 'ap8_failure', 'reason': 'initializeController'}],
                          'cleanup': {'owned_descendants_zero': complete, 'process_group_empty': complete}}
                def supervise(*args, checkpoint, **kwargs):
                    checkpoint('supervisor_cleanup', result)
                    checkpoint('supervisor_containment', result)
                    return result
                with patch.object(fixture.owner.runtime, 'supervise', side_effect=supervise), patch.object(fixture, 'write_atomic', side_effect=OSError('report disk failed')):
                    with self.assertRaises(fixture.owner.SessionError) as caught:
                        fixture.inspect(types.SimpleNamespace(run_id='3' * 32), pathlib.Path(directory) / 'results')
                self.assertEqual(caught.exception.record['observation']['records'][0]['reason'], 'initializeController')
                self.assertEqual(caught.exception.record['containment_confirmed'], complete)
                self.assertEqual(len(caught.exception.record['reporting_errors']), 3)

if __name__ == '__main__':
    unittest.main()

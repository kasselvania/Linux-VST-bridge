"""Run the product's fixed bootstrap with this actual host interpreter."""
import hashlib
import json
import os
from pathlib import Path
import py_compile
import subprocess
import sys
import tempfile
import unittest

BOOTSTRAP = Path(__file__).with_name("bootstrap.py").read_text()


class SourceBootstrap(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.package = self.root / "package"
        self.package.mkdir()
        self.session = self.package / "session.py"
        self.ownership = self.package / "ownership.py"
        self.ownership.write_text("VALUE = 'exact verified ownership'\n")
        self.session.write_text("""import json, os, sys, ownership
print(json.dumps({'session': __file__, 'ownership': ownership.__file__,
 'value': ownership.VALUE, 'argv': sys.argv, 'path': sys.path,
 'main_module': sys.modules['__main__'].__file__,
 'display': os.environ.get('DISPLAY'), 'scheduler': os.environ.get('LVB_AUDIO_SCHEDULER'),
 'isolated': sys.flags.isolated, 'no_site': sys.flags.no_site,
 'dont_write_bytecode': sys.dont_write_bytecode}))
""")

    def command(self, *args):
        sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
        return [sys.executable, '-I', '-S', '-B', '-c', BOOTSTRAP,
                str(self.session), sha(self.session), str(self.ownership), sha(self.ownership), *args]

    def run_bootstrap(self, *args, command=None, **kwargs):
        return subprocess.run(command or self.command(*args), capture_output=True,
                              text=True, timeout=10, **kwargs)

    def test_exact_artifacts_arguments_and_environment_ignore_hostile_imports(self):
        hostile = self.root / 'hostile'
        hostile.mkdir()
        (hostile / 'ownership.py').write_text("raise RuntimeError('ambient ownership ran')")
        (hostile / 'sitecustomize.py').write_text("raise RuntimeError('ambient site ran')")
        site = self.root / 'user' / 'lib' / f'python{sys.version_info.major}.{sys.version_info.minor}' / 'site-packages'
        site.mkdir(parents=True)
        (site / 'ownership.py').write_text("raise RuntimeError('user ownership ran')")
        (site / 'usercustomize.py').write_text("raise RuntimeError('user site ran')")
        environment = dict(os.environ, PYTHONPATH=str(hostile), PYTHONUSERBASE=str(self.root / 'user'),
                           DISPLAY=':fixture', LVB_AUDIO_SCHEDULER='exact scheduler handoff')
        result = self.run_bootstrap('--graphics-settings-v1', 'job with spaces.json',
                                    cwd=hostile, env=environment)
        self.assertEqual(result.returncode, 0, result.stderr)
        value = json.loads(result.stdout)
        self.assertEqual(value['session'], str(self.session))
        self.assertEqual(value['main_module'], str(self.session))
        self.assertEqual(value['ownership'], str(self.ownership))
        self.assertEqual(value['value'], 'exact verified ownership')
        self.assertEqual(value['argv'], [str(self.session), '--graphics-settings-v1', 'job with spaces.json'])
        self.assertEqual((value['display'], value['scheduler']), (':fixture', 'exact scheduler handoff'))
        self.assertEqual((value['isolated'], value['no_site'], value['dont_write_bytecode']), (1, 1, True))
        self.assertNotIn(str(hostile), value['path'])
        self.assertFalse(list(self.root.rglob('*.pyc')))

    def test_both_sources_verified_before_either_executes(self):
        marker = self.root / 'executed'
        self.ownership.write_text(f"from pathlib import Path\nPath({str(marker)!r}).touch()\n")
        for changed in (self.session, self.ownership):
            with self.subTest(changed=changed.name):
                original = changed.read_bytes()
                command = self.command()
                changed.write_bytes(original + b'\n# changed after Rust verification\n')
                result = self.run_bootstrap(command=command)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('supervisor source changed', result.stderr)
                self.assertFalse(marker.exists())
                changed.write_bytes(original)

    def test_declared_range_probe_and_refusal(self):
        command = [sys.executable, '-I', '-S', '-B', '-c', BOOTSTRAP, '--probe']
        result = subprocess.run(command, capture_output=True, text=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), 'cpython-source-v1')
        for version in ((3, 13), (3, 14), (3, 15)):
            command[5] = f'import sys; sys.version_info={version!r}; exec({BOOTSTRAP!r})'
            result = subprocess.run(command, capture_output=True, text=True, timeout=10)
            self.assertEqual((result.returncode, result.stdout.strip()), (0, 'cpython-source-v1'),
                             f'{version}: {result.stderr}')
        for version in ((3, 12), (3, 16), (4, 0)):
            command[5] = f'import sys; sys.version_info={version!r}; exec({BOOTSTRAP!r})'
            result = subprocess.run(command, capture_output=True, text=True, timeout=10)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('CPython >=3.13,<3.16', result.stderr)

    def test_non_cpython_refuses_before_loading_sources(self):
        code = 'import sys, types; sys.implementation=types.SimpleNamespace(name="pypy"); exec(' + repr(BOOTSTRAP) + ')'
        result = subprocess.run([sys.executable, '-I', '-S', '-B', '-c', code, '--probe'],
                                capture_output=True, text=True, timeout=10)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('CPython >=3.13,<3.16', result.stderr)

    def test_original_bytecode_shape_still_loads_its_adjacent_ownership(self):
        for source in (self.session, self.ownership):
            py_compile.compile(str(source), cfile=str(source.with_suffix('.pyc')), doraise=True)
            source.unlink()
        result = subprocess.run([sys.executable, str(self.session.with_suffix('.pyc')), 'old-job'],
                                capture_output=True, text=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        value = json.loads(result.stdout)
        self.assertEqual(value['ownership'], str(self.ownership.with_suffix('.pyc')))
        self.assertEqual(value['argv'], [str(self.session.with_suffix('.pyc')), 'old-job'])


if __name__ == '__main__':
    unittest.main()

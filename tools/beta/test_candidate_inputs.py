"""The release input path must consume the shipped reusable kit, not a catalogue."""
import contextlib
import hashlib
import io
import json
import pathlib
import runpy
import subprocess
import sys
import tempfile
import unittest
import zipfile
from unittest.mock import patch

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / 'pkg0'))
import assemble
import rust_notices


class CandidateInputs(unittest.TestCase):
    def test_reusable_kit_reaches_package_roster_without_any_exact_plugin_entry(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            source = root / 'source'
            source.mkdir()
            files = {
                'bridge-manager/target/x86_64-unknown-linux-gnu/release/linux-vst-bridge': b'\x7fELFmanager',
                'manager-ui/target/x86_64-unknown-linux-gnu/release/linux-audio-compatibility-manager': b'\x7fELFfrontend',
                'bridge-manager/runtime/session.py': b'# fixture supervisor\n',
                'bridge-manager/runtime/ownership.py': b'# fixture ownership\n',
                'bridge-manager/src/operator_model.rs': b'pub const OPERATOR_SCHEMA: u32 = 16;\n',
                'compatibility/arturia-pure-lofi.json': b'{}',
                'COPYRIGHT.md': b'First-party fixture',
            }
            for name, data in files.items():
                path = source / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(data)
            git = lambda *args: subprocess.check_output(['git', '-C', str(source), *args], text=True).strip()
            git('init', '-q')
            git('add', '.')
            git('-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid',
                'commit', '-qm', 'Frozen candidate source')
            windows = root / 'windows'
            windows.mkdir()
            host = b'MZfirst-party-host'
            host_source = b'{"schema":1}'
            (windows / 'wf0-factory-probe.exe').write_bytes(host)
            (windows / 'host-source-manifest.json').write_bytes(host_source)
            fixture = root / 'reference.vst3'
            fixture.write_bytes(b'MZfirst-party-reference')
            engine = b'\x7fELFone-engine-with-no-plug-in-ids'
            sha = lambda data: hashlib.sha256(data).hexdigest()
            entries = {
                'libap2_backend.a': b'backend',
                'tools/mf3/native_builder.py': b'# builder',
                'tools/ap8_descriptor.py': b'# descriptor',
                'runtime/host.exe': host,
                'runtime/host-source-manifest.json': host_source,
                'prebuilt/engine.so': engine,
                **{'licenses/' + name + '.txt': b'SDK fixture license'
                   for name in ('vst3sdk', 'base', 'pluginterfaces', 'public.sdk')},
            }
            entries['prebuilt/index.json'] = json.dumps(dict(schema=3,
                engine='prebuilt/engine.so', engine_sha256=sha(engine),
                descriptor_schema=1, maximum_bridge_frames=1024,
                audio_completion_contract=1, loaded_engine_admission_contract=1,
                native_sources={})).encode()
            recipe = dict(schema=4, source_commit=git('rev-parse', 'HEAD'),
                sdk=assemble.KIT_SDK, sdk_runtime=assemble.KIT_SDK_RUNTIME,
                files={name: sha(data) for name, data in entries.items()})
            kit = root / 'kit.zip'
            with zipfile.ZipFile(kit, 'w') as archive:
                for name, data in entries.items():
                    archive.writestr(name, data)
                archive.writestr('recipe.json', json.dumps(recipe))
            build = root / 'build'
            build.mkdir()

            def notices(_source, output):
                for name in ('Rust-Dependency-Notices.txt', 'Rust-Dependency-Inventory.json'):
                    (output / name).write_text('fixture')
                return []

            argv = [str(HERE / 'candidate_inputs.py'), '--build-root', str(build),
                '--source', str(source), '--windows-package', str(windows),
                '--kit', str(kit), '--fixture', str(fixture), '--version', '0.12.0fixture',
                '--epoch', '1']
            with patch.object(sys, 'argv', argv), patch.object(rust_notices, 'capture', notices), \
                    contextlib.redirect_stdout(io.StringIO()):
                runpy.run_path(argv[0], run_name='__main__')
            spec = json.loads((build / 'package-spec-0.12.0fixture.json').read_text())
            assemble.validate(spec, source)
            proxies = [row for row in spec['files'] if row['kind'] == 'proxy']
            self.assertEqual(len(proxies), 1)
            self.assertEqual(proxies[0]['destination'], 'usr/lib/linux-vst-bridge/proxy/ReusableEngine.so')
            self.assertEqual(pathlib.Path(proxies[0]['source']).read_bytes(), engine)
            guide = (build / 'inputs-0.12.0fixture/START_HERE.html').read_text()
            self.assertNotIn('Supported prebuilt proxy metadata', guide)
            self.assertIn('reusable native engine', guide)
            self.assertEqual(git('status', '--porcelain'), '')


if __name__ == '__main__':
    unittest.main()

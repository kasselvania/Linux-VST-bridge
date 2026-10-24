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
import frontend_build


class CandidateInputs(unittest.TestCase):
    def exercise_candidate(self, composite=False):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            source = root / 'source'
            source.mkdir()
            files = {
                'crates/cli/target/x86_64-unknown-linux-gnu/release/linux-vst-bridge': b'\x7fELFmanager',
                'manager-ui/target/x86_64-unknown-linux-gnu/release/linux-audio-compatibility-manager': b'\x7fELFfrontend',
                'bridge-manager/runtime/session.py': b'# fixture supervisor\n',
                'bridge-manager/runtime/ownership.py': b'# fixture ownership\n',
                'crates/model/src/operator_model.rs': b'pub const OPERATOR_SCHEMA: u32 = 16;\n',
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

            frontend_record = None
            installer_record = None
            frontend_output = root / 'frontend-build'
            if composite:
                frontend_output.mkdir()
                (frontend_output / frontend_build.BINARY).write_bytes(b'\x7fELFqualified-live-frontend')
                frontend_record = dict(schema=1,
                    bridge_source=dict(head=git('rev-parse','HEAD'),tree=git('rev-parse','HEAD^{tree}')),
                    frontend_source=dict(head='a'*40,tree='b'*40,manifest='app/Cargo.toml',
                        manifest_sha256='c'*64,cargo_lock_sha256='d'*64),
                    target=frontend_build.TARGET,binary=frontend_build.BINARY,features=['live'],
                    portable_public_key='e'*64,portable_key_class='internal_test',
                    artifact_sha256=sha((frontend_output/frontend_build.BINARY).read_bytes()))
                (frontend_output/'frontend-build.json').write_text(json.dumps(frontend_record))
                (frontend_output/frontend_build.INSTALLER).write_bytes(b'\x7fELFqualified-native-installer')
                installer_record = {**frontend_record, 'binary':frontend_build.INSTALLER,
                    'artifact_sha256':sha((frontend_output/frontend_build.INSTALLER).read_bytes())}

            def notices(_source, output, **kwargs):
                if composite:
                    self.assertEqual(kwargs,dict(frontend_source=root/'private',frontend_build=frontend_record,
                                                 installer_build=installer_record))
                else:
                    self.assertEqual(kwargs,{})
                for name in ('Rust-Dependency-Notices.txt', 'Rust-Dependency-Inventory.json'):
                    (output / name).write_text('fixture')
                return []

            argv = [str(HERE / 'candidate_inputs.py'), '--build-root', str(build),
                '--source', str(source), '--windows-package', str(windows),
                '--kit', str(kit), '--fixture', str(fixture), '--version', '0.12.0fixture',
                '--epoch', '1']
            if composite:
                argv += ['--frontend-build',str(frontend_output/'frontend-build.json'),
                         '--frontend-source',str(root/'private')]
            with patch.object(sys, 'argv', argv), patch.object(rust_notices, 'capture', notices), \
                    patch.object(frontend_build,'verify_pair',return_value=(frontend_record,installer_record)) as verified, \
                    contextlib.redirect_stdout(io.StringIO()):
                runpy.run_path(argv[0], run_name='__main__')
            spec = json.loads((build / 'package-spec-0.12.0fixture.json').read_text())
            assemble.validate(spec, source)
            frontend_row=next(row for row in spec['files'] if row['kind']=='frontend')
            if composite:
                verified.assert_called_once()
                self.assertEqual(frontend_row['frontend_build'],frontend_record)
                self.assertNotIn('build_head',frontend_row)
                self.assertEqual(pathlib.Path(frontend_row['source']).read_bytes(),b'\x7fELFqualified-live-frontend')
                compliance=json.loads((build/'inputs-0.12.0fixture/COMPLIANCE_MANIFEST.json').read_text())
                self.assertEqual(compliance['frontend_build'],frontend_record)
                self.assertEqual(compliance['installer_build'],installer_record)
                self.assertEqual((build/'inputs-0.12.0fixture'/frontend_build.INSTALLER).read_bytes(),
                                 b'\x7fELFqualified-native-installer')
                self.assertEqual(json.loads((build/'inputs-0.12.0fixture/installer-build.json').read_bytes()),
                                 installer_record)
            else:
                verified.assert_not_called()
                self.assertEqual(frontend_row['build_head'],git('rev-parse','HEAD'))
            proxies = [row for row in spec['files'] if row['kind'] == 'proxy']
            self.assertEqual(len(proxies), 1)
            self.assertEqual(proxies[0]['destination'], 'usr/lib/linux-vst-bridge/proxy/ReusableEngine.so')
            self.assertEqual(pathlib.Path(proxies[0]['source']).read_bytes(), engine)
            self.assertEqual(spec['external_runtime'], {
                'id':'managed-ge-proton11-7-slr4-20260805-r7',
                'manifest_sha256':sha(b'c5448b76a230384e2d7bc6beb5ccb97bafb7e2c3b6c527cb03a1a546bbcb00a0\n3226d8234e7c0542ee767837832bfb1dad5e5e2dc944ec97eb221b437f6b9349\n988fb967608a8c116022c27822647b7a7eda50a283a9e53ee2f22fe299b60f6c\n4ca51dec54f6085bd2050d8714018c3ba95bae7c00277de31e66d2c3ec7c9058\nca86baa047d98504faae52af1118403ff4860cf4aae039bdce68113f18360bf3\n'),
            })
            notices=(build/'inputs-0.12.0fixture/THIRD_PARTY_NOTICES.txt').read_text()
            self.assertIn('runtime-uia-r4/uia-guard.tar.gz',notices)
            self.assertIn('runtime-msi-r5/msi-reader.tar.gz',notices)
            self.assertIn('runtime-touch-r7/x11-touch.tar.gz',notices)
            self.assertIn('complete corresponding patched Wine source',notices)
            guide = (build / 'inputs-0.12.0fixture/START_HERE.html').read_text()
            self.assertNotIn('Supported prebuilt proxy metadata', guide)
            self.assertIn('Install and connect to your DAW', guide)
            self.assertIn('all newly installed VST3 plug-ins', guide)
            self.assertIn('Optional experience reports and checks', guide)
            self.assertIn('When Setup offers Add plug-ins through a vendor application', guide)
            self.assertIn('close the application normally', guide)
            self.assertIn('waits for its remaining helpers to finish', guide)
            self.assertIn('Unchanged plug-ins in the same installation are checked again', guide)
            self.assertIn('then automatically finds and connects new plug-ins', guide)
            self.assertNotIn('automatic continuation from it is not included', guide)
            self.assertNotIn('inside a vendor application is still a missing integration', guide)
            self.assertEqual(git('status', '--porcelain'), '')

    def test_reusable_kit_reaches_package_roster_without_any_exact_plugin_entry(self):
        self.exercise_candidate()

    def test_qualified_frontend_uses_standard_role_with_both_source_inputs(self):
        self.exercise_candidate(composite=True)


if __name__ == '__main__':
    unittest.main()

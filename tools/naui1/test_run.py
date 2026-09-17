import copy
import json
import os
import pathlib
import tempfile
import unittest
from unittest.mock import patch
from common import Refusal, canonical, decode, digest, publish, read
from package import FILES, verify
import run
from fixtures import application, source_records, log


class PackageTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(); self.addCleanup(self.tmp.cleanup)
        self.root = pathlib.Path(self.tmp.name).resolve(); self.hashes = {}
        for name in FILES:
            b = b'owned '+name.encode(); (self.root/name).write_bytes(b); (self.root/name).chmod(0o400)
            self.hashes[name] = digest(b)
        self.seal = {'schema': 1, 'source_head': 'a'*40, 'source_tree': 'b'*40, 'files': self.hashes}
        publish(self.root/'seal.json', self.seal)

    def test_exact_package_and_each_changed_missing_file(self):
        verify(self.root)
        for name in FILES:
            p = self.root/name; b = p.read_bytes(); p.chmod(0o600); p.write_bytes(b'x'); p.chmod(0o400)
            with self.assertRaises(Refusal): verify(self.root)
            p.unlink()
            with self.assertRaises(Refusal): verify(self.root)
            p.write_bytes(b); p.chmod(0o400)

    def test_extra_file_and_unsealed_read_write(self):
        (self.root/'extra').write_text('x')
        with self.assertRaises(Refusal): verify(self.root)
        (self.root/'extra').unlink(); (self.root/'run.py').chmod(0o600)
        with self.assertRaises(Refusal): verify(self.root)

    def test_duplicate_keys_and_missing_required_set_even_if_resealed(self):
        with self.assertRaises(Refusal): decode(b'{"schema":1,"schema":1}')
        p = self.root/'seal.json'; p.unlink(); self.hashes.pop('run.py'); (self.root/'run.py').unlink()
        publish(p, self.seal)
        with self.assertRaises(Refusal): verify(self.root)

    def test_no_replace_preserves_existing_bytes_and_partial_cleanup(self):
        before = read(self.root/'seal.json')
        with self.assertRaises(FileExistsError): publish(self.root/'seal.json', {'changed': True})
        self.assertEqual(before, read(self.root/'seal.json'))
        self.assertFalse(list(self.root.glob('.naui1-*')))

    def test_symlink_component_and_leaf_rejected(self):
        p = self.root/'run.py'; p.unlink(); p.symlink_to(self.root/'common.py')
        with self.assertRaises((Refusal, OSError)): verify(self.root)


class RunTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(); self.addCleanup(self.tmp.cleanup)
        self.home = pathlib.Path(self.tmp.name).resolve(); self.m = decode(pathlib.Path(run.__file__).with_name('input.json').read_bytes())
        self.root = self.home/self.m['managed_relation']; self.env = self.root/'environments'/self.m['environment']; self.d = self.root/'onboarding'/self.m['environment']
        self.env.mkdir(parents=True); self.d.mkdir(parents=True)
        app = application(self.env/'compatdata/pfx/drive_c')
        self.tx, self.result, self.identity, self.image = source_records()
        self.result.update(schema=2, state='completed', owned_live=0, cleanup_confirmed=True)
        self.result['transaction']['launch_binding']['artifact_sha256'] = self.m['installer_sha256']
        self.result['transaction']['launch_binding']['size'] = self.m['installer_size']
        r = self.tx['windows_trace']['processes'][0]; r['image_identity'] = {'sha256': self.m['installer_sha256'], 'size': self.m['installer_size']}
        self.tx['diagnostics']['runner'].update(capacity=262144, record_capacity=256, retained_bytes=0)
        self.result['transaction']['diagnostics'] = self.tx['diagnostics']
        self.tx['durable_installation'] = {'delta': {'logs': {'added': [], 'changed': []}}}
        er = {'id': self.m['environment'], 'root': str(self.env)}
        self.write(self.env/'environment.json', er)
        self.write(self.d/'record.json', {'id': self.m['environment'], 'environment': er, 'installer': self.m['installer_sha256'], 'published': False, 'installation_operation': self.m['operation']})
        self.sw = {}
        for k in self.m['installed_artifacts']:
            p = self.root/(k+'.bin'); b = (k+'bytes').encode(); p.write_bytes(b)
            self.sw[k] = {'path': str(p), 'sha256': digest(b)}; self.m['installed_artifacts'][k] = digest(b)
        self.m['software_sha256'] = self.write(self.root/'software.json', self.sw)
        self.m['result_sha256'] = self.write(self.d/(self.m['operation']+'-result.json'), self.result)
        self.m['transaction_sha256'] = self.write(self.d/(self.m['operation']+'-transaction-private.json'), self.tx)
        self.snap = {'schema': 5, 'system': {'service': 'active', 'keepers': 2, 'cleanup_unconfirmed': False, 'dsp': 0, 'maintenance': 0, 'pending_transactions': 0, 'stale_transports': 0}, 'capture': {'armed': False, 'active_retention': 0}, 'products': [], 'onboarding': [], 'operation': {'state': 'completed'}}
        self.prior = {'retained': {}, 'projects': {}, 'predecessor_files': {}, 'links': {}}
        install = self.home/'.cache/linux-vst-bridge/is4-install-8fb2f20'; install.mkdir(parents=True)
        self.m['preinstallation_sha256'] = self.write(install/'before-install.json', self.prior)

    @staticmethod
    def write(path, value):
        b = canonical(value); path.write_bytes(b); return digest(b)

    def mocked_process(self, args, **kwargs):
        import subprocess
        if args == [self.sw['manager']['path'], 'operator', 'snapshot']:
            return subprocess.CompletedProcess(args, 0, canonical(self.snap), b'')
        if args == ['systemctl', '--user', 'show', 'linux-vst-bridge-installer-'+self.m['operation']+'.service', '--property=ActiveState', '--value']:
            return subprocess.CompletedProcess(args, 0, b'inactive\n', b'')
        self.fail('Unexpected process launch')

    def test_custody_and_every_scalar_manifest_or_record_drift(self):
        run.validate_manifest(self.m)
        with patch.object(run.subprocess, 'run', side_effect=self.mocked_process):
            run.custody(self.m, self.home)
            for key in ('result_sha256', 'transaction_sha256', 'software_sha256', 'environment', 'operation', 'installer_sha256', 'installer_size'):
                bad = copy.deepcopy(self.m)
                bad[key] = bad[key]+1 if type(bad[key]) is int else '0'*len(bad[key])
                with self.subTest(key=key), self.assertRaises((Refusal, OSError)):
                    run.custody(bad, self.home)

    def test_active_or_capture_refuses_before_vendor_read(self):
        for key, value in [('service', 'inactive'), ('dsp', 1), ('maintenance', 1), ('keepers', 1), ('cleanup_unconfirmed', True), ('pending_transactions', 1), ('stale_transports', 1)]:
            s = copy.deepcopy(self.snap); s['system'][key] = value
            with self.assertRaises(Refusal): run.idle(s)
        self.snap['capture']['armed'] = True
        with self.assertRaises(Refusal): run.idle(self.snap)

    def test_input_cannot_choose_paths_commands_or_extra_fields(self):
        for key in ('command', 'args', 'path', 'environment_value', 'pid', 'token'):
            bad = copy.deepcopy(self.m); bad[key] = 'untrusted'
            with self.assertRaises(Refusal): run.validate_manifest(bad)
        bad = copy.deepcopy(self.m); bad['managed_relation'] = '/tmp'
        with self.assertRaises(Refusal): run.validate_manifest(bad)

    def test_single_full_offline_pass_and_no_repeat(self):
        # Actual orchestration with generated prefix/records and only the two
        # closed readback commands replaced. Any vendor launch fails this test.
        source = self.home/'package'; source.mkdir()
        for name in FILES:
            src = pathlib.Path(run.__file__).parent/name
            b = canonical(self.m)+b'\n' if name == 'input.json' else src.read_bytes()
            (source/name).write_bytes(b); (source/name).chmod(0o400)
        seal = {'schema': 1, 'source_head': 'a'*40, 'source_tree': 'b'*40,
                'files': {name: digest((source/name).read_bytes()) for name in FILES}}
        publish(source/'seal.json', seal)
        with patch.object(run, '__file__', str(source/'run.py')), patch.object(run.pathlib.Path, 'home', return_value=self.home), patch.object(run.sys, 'argv', ['run.py']), patch.object(run.subprocess, 'run', side_effect=self.mocked_process), patch('builtins.print'):
            run.main()
            out = self.home/'.cache/linux-vst-bridge/naui1-aaaaaaaaaaaa/result.json'
            result = decode(out.read_bytes())
            self.assertEqual(result['preservation']['commercial_launches'], 0)
            self.assertEqual(result['disposition'], 'NAUI1_ELECTRON_IDENTITY_ONLY_CAUSE_UNRESOLVED')
            with self.assertRaises(FileExistsError): run.main()

    def test_identity_refusal_and_records_refusal_preserve_unresolved(self):
        drive = self.env/'compatdata/pfx/drive_c'
        self.tx['windows_trace']['processes'].reverse()
        ans = run.inspect(drive, self.tx, self.result, {})
        self.assertEqual(ans[-1], 'NAUI1_ELECTRON_IDENTITY_ONLY_CAUSE_UNRESOLVED')
        (drive/'Program Files/Native Instruments/Native Access/Native Access.exe').unlink()
        self.assertEqual(run.inspect(drive, self.tx, self.result, {})[-1], 'NAUI1_APPLICATION_IDENTITY_UNRESOLVED')


if __name__ == '__main__': unittest.main()

"""Focused metadata admission and unchanged frozen-function regressions."""
import ast
import copy
import hashlib
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import types
import unittest
from unittest.mock import patch
TOOLS=pathlib.Path(__file__).resolve().parent
sys.path.insert(0,str(TOOLS))
import pc0_diagnostic_runtime as d


def reference_observation():
    baseline=d.lock_api()['expected_lock_manifest']()
    observed=copy.deepcopy(baseline)
    observed['files'][-2]['size']=691
    observed['files'][-2]['sha256']='06c0038ea6052c563cb1ca3d75c6ef8915aad2209ccd05541e18b95f93ae6151'
    apps={}
    for appid,(build,directory,depot,manifest,size) in d.APPS.items():
        apps[appid]={'selection':{'appid':appid,'universe':'1','buildid':build,
            'installdir':directory,'SizeOnDisk':size,'InstalledDepots':{depot:{'manifest':manifest,'size':size}},
            'UserConfig':{},'MountedConfig':{}},'bookkeeping':{'state_flags':'4',
            'target_build':None,'scheduled_update':'0','pending_download_bytes':'0'}}
    selection={'runner':baseline['runner'],'runtime':baseline['runtime'],
        'files':[r for r in baseline['files'] if not r['safe_path'].startswith('steamapps/')],
        'applications':{k:v['selection'] for k,v in apps.items()}}
    return {'schema':d.SCHEMA,'baseline_contract_sha256':d.BASELINE,
        'launch_critical_manifest_sha256':d.digest(observed),'declared_inputs_sha256':d.digest(selection),
        'observed_manifest':observed,'applications':apps}


def vdf(value):
    def body(obj):
        return '\n'.join(json.dumps(k)+' '+('{\n'+body(v)+'\n}' if isinstance(v,dict) else json.dumps(v)) for k,v in obj.items())
    return body(value).encode()

def app(appid='4628710'):
    value=copy.deepcopy(reference_observation()['applications'][appid]['selection'])
    value.update(name='Proton 11.0' if appid=='4628710' else 'Steam Linux Runtime 4.0',
        StateFlags='6' if appid=='4628710' else '4', LastOwner='1234', DownloadType='2',
        BytesDownloaded='0',BytesStaged='0',BytesToStage='0',StagingSize='0',UpdateResult='0',
        TargetBuildID='25118279',BytesToDownload='160',ScheduledAutoUpdate='1788604512')
    return value

class RuntimeTests(unittest.TestCase):
    def test_pending_metadata_preserves_installed_selection(self):
        before=d.application(vdf({'AppState':app()}),'4628710')
        a=app();a.update(LastOwner='9999',LastPlayed='999',ScheduledAutoUpdate='999')
        after=d.application(vdf({'AppState':a}),'4628710')
        self.assertEqual(before['selection'],after['selection'])
        self.assertNotEqual(before['bookkeeping'],after['bookkeeping'])
        self.assertNotIn('LastOwner',json.dumps(after))

    def test_rejects_changed_selection_active_update_and_unknown_fields(self):
        for key,value in [('appid','9'),('buildid','25118279'),('installdir','Other'),
            ('InstalledDepots',{'4628711':{'manifest':'9','size':'1445063147'}}),
            ('UserConfig',{'LaunchOptions':'override'}),('MountedConfig',{'language':'new'}),
            ('StateFlags','1028'),('BytesDownloaded','1'),('BytesStaged','1'),
            ('StagingSize','1'),('BytesToStage','1'),('UpdateResult','2'),('UnknownSetting','0')]:
            with self.subTest(key=key):
                a=app();a[key]=value
                with self.assertRaises(RuntimeError):d.application(vdf({'AppState':a}),'4628710')

    def test_supervision_error_is_useful_and_private(self):
        self.assertIn("missing runtime identity",d.sanitized_supervision_error(TypeError("missing runtime identity")))
        text=d.sanitized_supervision_error(RuntimeError("/home/private/file token=secret user@example.com 192.0.2.1 pid=999"))
        for private in ('/home/private','token=secret','user@example.com','192.0.2.1','pid=999'):
            self.assertNotIn(private,text)
        self.assertLessEqual(len(d.sanitized_supervision_error(RuntimeError('a'*1000))),768)

    def test_duplicate_or_malformed_vdf_rejected(self):
        for raw in [b'"AppState" { "appid" "1" "appid" "2" }',b'"AppState" {',
                    b'"AppState" {} garbage',b'"AppState" {} }',b'"AppState" "x"']:
            with self.subTest(raw=raw):
                with self.assertRaises(RuntimeError):d.parse_vdf(raw)

    def test_observed_digest_is_not_old_snapshot(self):
        value=reference_observation()
        self.assertNotEqual(d.validate_runtime_observation(value),d.BASELINE)
        value['launch_critical_manifest_sha256']=d.BASELINE
        with self.assertRaises(RuntimeError):d.validate_runtime_observation(value)

    def test_rejects_runtime_content_change_even_with_rehashed_snapshot(self):
        for field in ('sha256','mode','size'):
            value=reference_observation();value['observed_manifest']['files'][2][field]='bad'
            value['launch_critical_manifest_sha256']=d.digest(value['observed_manifest'])
            with self.assertRaises(RuntimeError):d.validate_runtime_observation(value)

    def test_real_files_exact_and_metadata_independent(self):
        with tempfile.TemporaryDirectory() as temp:
            root=pathlib.Path(temp); baseline=d.lock_api()['expected_lock_manifest']()
            specs=[]
            for base,name,raw in [('runner','proton',b'executable'),('runtime','run',b'runtime'),
                ('steamapps','appmanifest_4628710.acf',vdf({'AppState':app()})),
                ('steamapps','appmanifest_4183110.acf',vdf({'AppState':app('4183110')}))]:
                p=root/base/name;p.parent.mkdir(exist_ok=True);p.write_bytes(raw);p.chmod(0o755)
                specs.append(types.SimpleNamespace(base=base,relative=name,size=len(raw),mode='0755',sha256=hashlib.sha256(raw).hexdigest()))
            baseline['files']=[{'safe_path':s.base+'/'+s.relative,'type':'regular_file','size':s.size,'mode':s.mode,'sha256':s.sha256} for s in specs]
            api={'expected_lock_manifest':lambda:copy.deepcopy(baseline),'LOCK_FILES':specs,'lock_base':lambda base:root/base,
                'require_no_symlink_ancestors':lambda *a,**kw:None,'require_contained':lambda *a,**kw:None,
                'sha256_file':lambda p:hashlib.sha256(p.read_bytes()).hexdigest()}
            with patch.object(d,'lock_api',return_value=api):
                first=d.verify_diagnostic_runner()
                a=app();a['LastPlayed']='199';(root/'steamapps/appmanifest_4628710.acf').write_bytes(vdf({'AppState':a}))
                second=d.verify_diagnostic_runner()
                self.assertEqual(first['declared_inputs_sha256'],second['declared_inputs_sha256'])
                self.assertNotEqual(first['launch_critical_manifest_sha256'],second['launch_critical_manifest_sha256'])
                (root/'runner/proton').write_bytes(b'changed')
                with self.assertRaisesRegex(RuntimeError,'Deployed runtime input'):d.verify_diagnostic_runner()

    def test_frozen_process_functions_only_change_runtime_dependencies(self):
        path=TOOLS/'pc0_diagnostic_primitives.py'
        current={n.name:ast.get_source_segment(path.read_text(),n) for n in ast.parse(path.read_text()).body if isinstance(n,ast.FunctionDef)}
        for module,names in [('environment',['verify_environment','retire_environment','create_dx0_environment']),('supervise',['supervise']),('run',['validate_failure_diagnostic']),('normalize',['normalize_wa0_positive'])]:
            text=subprocess.check_output(['git','show','309b8918c128c0b9e6701d0453dc841a111d5ac5:tools/wf0-factory-census/'+module+'.py'],cwd=TOOLS.parent).decode()
            for node in ast.parse(text).body:
                if not isinstance(node,ast.FunctionDef) or node.name not in names:continue
                actual=current[node.name]
                actual=actual.replace('            "supervision_error": sanitized_supervision_error(supervision_error),\n','')
                actual=actual.replace('            "supervision_exception": exception_detail(supervision_error),\n','')
                # AP0 supplies only mode/stream and host-verification seams;
                # remove those selections to compare the unchanged PC0 path.
                actual=actual.replace(', checkpoint=None, profile=None, session_override=None)', ')')
                actual=actual.replace('session_override or secrets.token_hex(16)', 'secrets.token_hex(16)')
                actual=actual.replace(', verify_host=None)', ')')
                actual=actual.replace('(verify_host or verify_host_store)', 'verify_host_store')
                actual=actual.replace('(profile.command_vector if profile else command_vector)', 'command_vector')
                actual=actual.replace('profile.StreamState() if profile else StreamState()', 'StreamState()')
                actual=actual.replace('mode != PC0_MODE and profile is None', 'mode != PC0_MODE')
                actual=actual.replace('    cleanup_exception = None\n', '')
                actual=actual.replace('            cleanup_exception = error\n', '')
                actual=actual.replace('    starts, completes = validate_wa0_event_order(records, pre_setup=pre_setup)',
                                      '    validate_wa0_event_order(records, pre_setup=pre_setup)')
                actual=actual.replace('len(starts)', 'len(expected_calls)').replace('len(completes)', 'len(expected_calls)')
                actual=actual.replace(', *, runner_identity_sha256: str)',')').replace(', runner_identity_sha256: str)',')')
                actual=actual.replace(', runner_identity_sha256=runner_identity_sha256','')
                actual=actual.replace('"runner_identity_sha256": runner_identity_sha256','"runner_identity_sha256": RUNNER_DIGEST')
                actual=actual.replace('!= runner_identity_sha256','!= RUNNER_DIGEST')
                actual=actual.replace('    runner_identity = verify_diagnostic_runner()\n    verify_environment(environment, runner_identity_sha256=runner_identity["launch_critical_manifest_sha256"])', '    verify_environment(environment)\n    runner_identity = verify_runner_identity()')
                actual=actual.replace('verify_environment(environment, runner_identity_sha256=runner_identity["launch_critical_manifest_sha256"])','verify_environment(environment)')
                actual=actual.replace('verify_diagnostic_runner()', 'verify_runner_identity()')
                actual=actual.replace(', expected_runner_identity_sha256: str)',' )').replace('str )','str)')
                actual=actual.replace('!= expected_runner_identity_sha256','!= RUNNER_DIGEST')
                if node.name == 'supervise':
                    tree=ast.parse(actual)
                    class StripCapture(ast.NodeTransformer):
                        def visit_If(self, n):
                            if ast.unparse(n.test) == 'checkpoint is not None':
                                return None
                            return self.generic_visit(n)
                        def visit_Try(self, n):
                            n=self.generic_visit(n)
                            if not n.handlers and not n.finalbody:
                                return n.body
                            return n
                    actual=ast.unparse(StripCapture().visit(tree))
                with self.subTest(function=node.name):
                    self.assertEqual(ast.dump(ast.parse(actual)),ast.dump(ast.parse(ast.get_source_segment(text,node))))

if __name__=='__main__':unittest.main()

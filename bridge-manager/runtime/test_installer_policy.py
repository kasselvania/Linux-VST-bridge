import copy, hashlib, inspect, json, pathlib, tempfile, unittest
from unittest.mock import patch
import session

class PolicyTests(unittest.TestCase):
    def test_unselected_and_inherited_preserve_environment_exactly(self):
        for value in (None, '', 'foo=n,b; bar=b;UIAutomationCore=', 'PowerShell.exe=b;foo=n'):
            env={'HOME':'private','WINEDEBUG':'-all'}
            if value is not None:env['WINEDLLOVERRIDES']=value
            before=json.dumps(env)
            changed,receipt=session.installer_policy_environment(None,env)
            self.assertIs(changed,env);self.assertIsNone(receipt)
            changed,receipt=session.installer_policy_environment(self.policy('inherited'),env)
            self.assertEqual(json.dumps(changed),before);self.assertEqual(json.dumps(env),before)
            self.assertEqual(receipt['before'],receipt['after'])
    def policy(self,mode='intentionally_unavailable'):
        return {'schema':1,'operation':'ab'*16,'environment':{'id':'cd'*16,'revision':1},
                'software_sha256':'ef'*32,'windows_scripting':{'powershell':mode}}
    def test_only_exact_rule_changes_and_unrelated_bytes_survive(self):
        cases={None:'powershell.exe=', '':'powershell.exe=',
               'foo=n,b;bar=b':'foo=n,b;bar=b;powershell.exe=',
               'foo=n;':'foo=n;powershell.exe=',
               'foo=n; PowerShell.ExE =n,b; BAR=b':'foo=n;powershell.exe=; BAR=b',
               'powershell=b;*powershell.exe=n':'powershell=b;*powershell.exe=n;powershell.exe=',
               'a,b=n;foo=':'a,b=n;foo=;powershell.exe='}
        for before,after in cases.items():
            self.assertEqual(session.installer_override_absence(before),after)
            env={} if before is None else {'WINEDLLOVERRIDES':before}
            changed,receipt=session.installer_policy_environment(self.policy(),env)
            self.assertEqual(changed['WINEDLLOVERRIDES'],after)
            self.assertEqual(receipt['after']['sha256'],hashlib.sha256(after.encode()).hexdigest())
            self.assertNotIn(after,json.dumps(receipt))
    def test_conflicting_duplicate_mixed_or_malformed_rules_refuse(self):
        for value in ('powershell.exe=n;POWERSHELL.EXE=b','powershell.exe=;powershell.exe=',
                      'foo,powershell.exe=n','powershell.exe','x\0y','x'*16385):
            with self.assertRaises(ValueError):session.installer_override_absence(value)
    def bound(self):
        def a(p):return {'path':str(p),'sha256':hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()}
        # Source-owned manager artifact sufficient for identity test; no launch.
        artifact=a(__file__)
        spec={'schema':3,'operation':'ab'*16,'environment':{'id':'cd'*16,'revision':1},'installer':artifact,'format':'pe_executable','installer_launch':artifact}
        policy=self.policy();policy.update(installer=artifact,owners={'manager':artifact,
            'supervisor':a(session.__file__),'ownership':a(session.sys.modules['ownership'].__file__)})
        policy['format']='pe_executable';policy['installer_launch']=artifact
        policy['software']=dict(policy['owners'],installer_launch=artifact)
        policy['software_sha256']=hashlib.sha256(json.dumps(policy['software'],sort_keys=True,separators=(',',':')).encode()).hexdigest()
        spec['installer_capability']=policy
        return spec
    def test_operation_environment_artifact_and_executing_generation_exact(self):
        good=self.bound();self.assertEqual(session.installer_policy_validate(good),good['installer_capability'])
        for key,bad in [('schema',2),('operation','ff'*16),('environment',{}),('installer',{})]:
            changed=copy.deepcopy(good);changed['installer_capability'][key]=bad
            with self.assertRaises((ValueError,RuntimeError)):session.installer_policy_validate(changed)
        for owner in ('manager','supervisor','ownership'):
            changed=copy.deepcopy(good);changed['installer_capability']['owners'][owner]['sha256']='a'*64
            with self.assertRaises((RuntimeError,ValueError)):session.installer_policy_validate(changed)
        changed=copy.deepcopy(good);changed['installer_capability']['owners']['supervisor']=good['installer']
        with self.assertRaises(ValueError):session.installer_policy_validate(changed)
    def test_closed_schema_and_missing_policy_refuse(self):
        good=self.bound()
        for mode in ('required_interpreter','powershell.exe=','unknown'):
            changed=copy.deepcopy(good);changed['installer_capability']['windows_scripting']['powershell']=mode
            with self.assertRaises(ValueError):session.installer_policy_validate(changed)
        for key in ('command','override','path','environment_variables'):
            changed=copy.deepcopy(good);changed['installer_capability'][key]='anything'
            with self.assertRaises(ValueError):session.installer_policy_validate(changed)
        good.pop('installer_capability')
        with self.assertRaises(ValueError):session.installer_policy_validate(good)
        good['schema']=2;self.assertIsNone(session.installer_policy_validate(good))
    def test_qualified_route_refuses_before_any_process_creation(self):
        good=self.bound()
        cases=[]
        for field,value in [('installer_launch',None),('format','msi_compound'),('format','other')]:
            bad=copy.deepcopy(good);bad[field]=value;cases.append(bad)
        for location in ('spec','policy','software'):
            bad=copy.deepcopy(good)
            obj=bad if location=='spec' else bad['installer_capability'] if location=='policy' else bad['installer_capability']['software']
            obj['installer_launch']={'path':__file__,'sha256':'0'*64}
            cases.append(bad)
        bad=copy.deepcopy(good);bad['installer_launch']=None
        bad['installer_capability']['installer_launch']=None
        bad['installer_capability']['software']['installer_launch']=None
        bad['installer_capability']['software_sha256']=hashlib.sha256(json.dumps(bad['installer_capability']['software'],sort_keys=True,separators=(',',':')).encode()).hexdigest()
        cases.append(bad)
        # Even a consistently rebound but changed artifact must verify its bytes.
        bad=copy.deepcopy(good);artifact={'path':__file__,'sha256':'0'*64}
        bad['installer_launch']=artifact;bad['installer_capability']['installer_launch']=artifact
        bad['installer_capability']['software']['installer_launch']=artifact
        bad['installer_capability']['software_sha256']=hashlib.sha256(json.dumps(bad['installer_capability']['software'],sort_keys=True,separators=(',',':')).encode()).hexdigest()
        cases.append(bad)
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp);(root/'operation.lock').touch()
            for n,bad in enumerate(cases):
                bad['environment']['root']=str(root);bad['installer_capability']['environment']=bad['environment'];bad['report']=str(root/f'result-{n}.json')
                with patch.object(session.subprocess,'Popen') as launch, patch.object(session.signal,'signal'):
                    self.assertFalse(session.install(bad));launch.assert_not_called()
                result=json.loads(pathlib.Path(bad['report']).read_text())
                self.assertIsNone(result['installer_capability']['effective'])
                self.assertFalse((root/(bad['operation']+'-installer-policy.private.json')).exists())
        ordinary=copy.deepcopy(good);ordinary['schema']=2;ordinary.pop('installer_capability');ordinary.pop('installer_launch')
        before=copy.deepcopy(ordinary);self.assertIsNone(session.installer_policy_validate(ordinary));self.assertEqual(ordinary,before)

    def test_policy_application_is_after_initialization_before_target(self):
        source=inspect.getsource(session.managed_install)
        self.assertLess(source.index("child=launch(base+['getcompatpath'"),source.index('installer_policy_environment(policy,launch_env)'))
        self.assertLess(source.index('installer_policy_environment(policy,launch_env)'),source.index("child=launch(argv,'target_runner')"))
        self.assertNotIn('WINEDLLOVERRIDES',source)

if __name__=='__main__':unittest.main()

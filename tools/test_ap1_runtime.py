"""AP1 selection uses the same verifier without promoting historical runtimes."""
import copy,hashlib,pathlib,sys,tempfile,types,unittest
from unittest.mock import patch
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parent))
import pc0_diagnostic_runtime as old
import ap1_runtime as new
from test_pc0_diagnostic_runtime import reference_observation,vdf

def observation():
    value=reference_observation();baseline=new.baseline()
    value['baseline_contract_sha256']=new.identity()
    value['observed_manifest']=baseline
    value['launch_critical_manifest_sha256']=old.digest(baseline)
    value['declared_inputs_sha256']=new.declared_inputs()
    for appid,(build,directory,depot,manifest,size) in new.APPS.items():
        value['applications'][appid]['selection']={'appid':appid,'universe':'1','buildid':build,'installdir':directory,'SizeOnDisk':size,
            'InstalledDepots':{depot:{'manifest':manifest,'size':size}},'UserConfig':{},'MountedConfig':{}}
    return value

class AP1RuntimeTests(unittest.TestCase):
    def test_bindings_are_separate_and_historical_defaults_unchanged(self):
        before=old.lock_api()['expected_lock_manifest']()
        self.assertEqual(old.lock_api()['expected_lock_digest'](),old.BASELINE)
        self.assertNotEqual(new.identity(),old.BASELINE)
        self.assertEqual(new.validate_observation(observation()),observation()['launch_critical_manifest_sha256'])
        with self.assertRaises(RuntimeError):old.validate_runtime_observation(observation())
        with self.assertRaises(RuntimeError):new.validate_observation(reference_observation())
        self.assertEqual(old.lock_api()['expected_lock_manifest'](),before)
        self.assertEqual(old.validate_runtime_observation(reference_observation()),reference_observation()['launch_critical_manifest_sha256'])

    def test_ap1_rejects_rehashed_changed_bytes_and_wrong_app(self):
        for path in ('runner/proton','runner/files/bin/wine','runtime/VERSIONS.txt'):
            value=observation();next(r for r in value['observed_manifest']['files'] if r['safe_path']==path)['sha256']='0'*64
            value['launch_critical_manifest_sha256']=old.digest(value['observed_manifest'])
            with self.assertRaises(RuntimeError):new.validate_observation(value)
        value=observation();value['applications']['4628710']['selection']['buildid']='24867889'
        with self.assertRaises(RuntimeError):new.validate_observation(value)

    def test_selected_application_still_rejects_active_update_and_settings(self):
        value=observation()['applications']['4628710']['selection']
        value.update(name='Proton 11.0',StateFlags='4')
        self.assertEqual(old.application(vdf({'AppState':value}),'4628710',applications=new.APPS)['selection']['buildid'],'25118279')
        for key,bad in [('StateFlags','1028'),('BytesDownloaded','1'),('buildid','24867889'),('UserConfig',{'LaunchOptions':'bad'}),('Unknown','0')]:
            changed={**value,key:bad}
            with self.subTest(key=key),self.assertRaises(RuntimeError):old.application(vdf({'AppState':changed}),'4628710',applications=new.APPS)

    def test_completed_update_counters_are_explicit_and_partial_states_rejected(self):
        app=observation()['applications']['4628710']['selection']
        app.update(name='Proton 11.0',StateFlags='4',UpdateResult='0',StagingSize='0',
            TargetBuildID='25118279',ScheduledAutoUpdate='0',BytesToDownload='160',
            BytesDownloaded='160',BytesToStage='747498077',BytesStaged='747498077')
        raw=vdf({'AppState':app})
        with self.assertRaises(RuntimeError):old.application(raw,'4628710',applications=new.APPS)
        self.assertEqual(old.application(raw,'4628710',applications=new.APPS,allow_completed_update=True)['selection']['buildid'],'25118279')
        for key,bad in [('BytesDownloaded','159'),('BytesStaged','747498076'),('StagingSize','1'),
                        ('UpdateResult','1'),('StateFlags','1028'),('StateFlags','6'),
                        ('TargetBuildID','25118280'),('ScheduledAutoUpdate','1')]:
            with self.subTest(key=key),self.assertRaises(RuntimeError):
                old.application(vdf({'AppState':{**app,key:bad}}),'4628710',applications=new.APPS,allow_completed_update=True)
        missing=dict(app);del missing['BytesStaged']
        with self.assertRaises(RuntimeError):old.application(vdf({'AppState':missing}),'4628710',applications=new.APPS,allow_completed_update=True)

    def test_actual_file_verifier_uses_explicit_selection_and_detects_drift(self):
        with tempfile.TemporaryDirectory() as temp:
            root=pathlib.Path(temp);baseline=new.baseline();legacy=copy.deepcopy(baseline);specs=[]
            for record in baseline['files']:
                base,relative=record['safe_path'].split('/',1)
                if base=='steamapps':
                    appid=relative.removeprefix('appmanifest_').removesuffix('.acf')
                    app=observation()['applications'][appid]['selection']
                    app.update(name='Proton 11.0' if appid=='4628710' else 'Steam Linux Runtime 4.0',StateFlags='4')
                    raw=vdf({'AppState':app})
                else:raw=record['safe_path'].encode()
                p=root/base/relative;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(raw);p.chmod(int(record['mode'],8))
                record.update(size=len(raw),sha256=hashlib.sha256(raw).hexdigest())
                specs.append(types.SimpleNamespace(base=base,relative=relative))
            api={'expected_lock_manifest':lambda:copy.deepcopy(legacy),'LOCK_FILES':specs,'lock_base':lambda base:root/base,
                 'require_no_symlink_ancestors':lambda *a,**k:None,'require_contained':lambda *a,**k:None,
                 'sha256_file':lambda p:hashlib.sha256(p.read_bytes()).hexdigest()}
            with patch.object(old,'lock_api',return_value=api):
                result=old.verify_diagnostic_runner(baseline=baseline,applications=new.APPS)
                old.validate_runtime_observation(result,baseline=baseline,applications=new.APPS)
                with self.assertRaises(RuntimeError):old.verify_diagnostic_runner()
                (root/'runner/proton').write_bytes(b'changed')
                with self.assertRaisesRegex(RuntimeError,'Deployed runtime input'):old.verify_diagnostic_runner(baseline=baseline,applications=new.APPS)

if __name__=='__main__':unittest.main()

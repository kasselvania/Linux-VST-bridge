"""Existing bundle-payload recovery, without a proprietary executable or launch."""
import copy,hashlib,json,pathlib,tempfile,unittest
from unittest.mock import patch
import session as s
import ownership
import test_dependency_owner as harness

class RecoveryInputs(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
  self.root=pathlib.Path(self.tmp.name).resolve();self.prior=self.root/'prior';self.prior.mkdir()
  self.drive=self.root/'compatdata/pfx/drive_c';self.drive.mkdir(parents=True)
  data=bytearray(256);data[:2]=b'MZ';data[60:64]=(64).to_bytes(4,'little');data[64:70]=b'PE\0\0\x64\x86'
  for name in ('Setup.exe','NTKDaemon.exe'):(self.drive/name).write_bytes(data)
  daemon=ownership.image_identity(self.drive/'NTKDaemon.exe')
  self.spec={'operation':'a'*32,'application_identity':'b'*64,'application':{'environment':{'root':str(self.root)}},'kind':'native_access_dependency','software':{'generation':'new'}}
  self.old=dict(self.spec,operation='c'*32,software={'generation':'old'})
  self.result={'operation':'c'*32,'state':'failed','error':'dependency_command_timeout','cleanup_confirmed':True,'owned_live':0,'dependency':{'ready_tested':False,'service_retirement_confirmed':True,'process_cleanup_confirmed':True,'forced_cleanup_used':False}}
  (self.prior/'spec.json').write_text(json.dumps(self.old));(self.prior/'result.json').write_text(json.dumps(self.result));(self.prior/'installer.log').write_bytes(b'private generated installer exit 100')
  self.q={'schema':1,'operation':'c'*32,'application_identity':'b'*64,'installer_sha256':daemon['sha256'],'installer_size':daemon['size'],'daemon':daemon,'sources':{}}
  self.seal()
 def seal(self):
  self.q['sources']={p.name:{'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'size':p.stat().st_size} for p in self.prior.iterdir()}
 def verify(self):return s.nad1_recovery_inputs(self.spec,self.prior,self.q,'Setup.exe','NTKDaemon.exe')
 def test_exact_recovery_crosses_software_update_but_grants_no_readiness(self):
  saved={p.name:p.read_bytes() for p in self.prior.iterdir()};r=self.verify()
  self.assertEqual(r['daemon'],self.q['daemon']);self.assertTrue(r['prior_failure_preserved']);self.assertFalse(r['installer_reexecuted']);self.assertNotIn('ready_tested',r)
  self.assertEqual(saved,{p.name:p.read_bytes() for p in self.prior.iterdir()})
 def test_every_retained_source_changed_missing_or_aliased_refuses(self):
  for name in self.q['sources']:
   p=self.prior/name;data=p.read_bytes()
   for mode in ('changed','missing','symlink','hardlink'):
    with self.subTest(name=name,mode=mode):
     p.unlink();outside=self.root/'outside';outside.write_bytes(data)
     if mode=='changed':p.write_bytes(data+b'x')
     if mode=='symlink':p.symlink_to(outside)
     if mode=='hardlink':p.hardlink_to(outside)
     with self.assertRaises((ValueError,OSError)):self.verify()
     if p.exists() or p.is_symlink():p.unlink()
     outside.unlink();p.write_bytes(data)
 def test_payload_and_installer_changes_refuse_without_launch_or_write(self):
  for name in ('Setup.exe','NTKDaemon.exe'):
   p=self.drive/name;saved=p.read_bytes();p.write_bytes(saved+b'x')
   with patch.object(s.subprocess,'Popen') as launch,patch.object(s,'nad1_publish') as write:
    with self.assertRaises(ValueError):self.verify()
    launch.assert_not_called();write.assert_not_called()
   p.write_bytes(saved)
 def test_missing_daemon_is_not_permission_to_reinstall(self):
  (self.drive/'NTKDaemon.exe').unlink()
  with self.assertRaises(FileNotFoundError):self.verify()
 def test_wrong_application_operation_environment_or_prior_kind_refuses(self):
  for key in ('application_identity','operation','kind','application'):
   old=copy.deepcopy(self.old);old[key]='wrong' if key!='application' else {'foreign':True}
   (self.prior/'spec.json').write_text(json.dumps(old));self.seal()
   with self.assertRaises(ValueError):self.verify()
  (self.prior/'spec.json').write_text(json.dumps(self.old));self.seal()
  self.spec['operation']=self.q['operation']
  with self.assertRaises(ValueError):self.verify()
 def test_unclean_prior_refuses_even_when_fixture_reseals_it(self):
  for field,value in [('state','completed'),('cleanup_confirmed',False),('owned_live',1),('service_retirement_confirmed',False),('process_cleanup_confirmed',False),('forced_cleanup_used',True)]:
   r=copy.deepcopy(self.result);target=r if field in r else r['dependency'];target[field]=value
   (self.prior/'result.json').write_text(json.dumps(r));self.seal()
   with self.assertRaises(ValueError):self.verify()
 def test_replacement_after_initial_hash_is_rechecked(self):
  original=ownership.image_identity
  def image(path):
   result=original(path);(self.prior/'installer.log').write_bytes(b'changed');return result
  with patch.object(ownership,'image_identity',side_effect=image),self.assertRaises(ValueError):self.verify()
 def test_production_python_and_rust_qualification_agree(self):
  p=pathlib.Path(__file__).resolve().parents[1]/'src/native_access_recovery.json'
  if not p.exists():self.skipTest('standalone staged runtime')
  self.assertEqual(s.NAD1_RECOVERY,json.loads(p.read_bytes()))
  self.assertEqual(s.NAD1_RECOVERY['daemon']['sha256'],'e20b3d30b72d6a12e0a37b5fbd1a5c21e0db9e53459b4aab165270f7f739343b')

class RecoveryLifecycle(unittest.TestCase):
 setUp=harness.OwnerTests.setUp
 existing=harness.OwnerTests.existing
 session_owner=harness.OwnerTests.session_owner
 def recovery(self):
  admitted=self.existing();command=self.owner.command;self.owner=self.session_owner();self.owner.command=command;return admitted
 def test_recovery_starts_and_retires_without_reinstall_or_direct_execution(self):
  admitted=self.recovery();self.owner.ensure(False,admitted);self.assertTrue(self.owner.retire())
  self.assertEqual(self.commands,['query','start','query','stop','query']);self.assertTrue(self.owner.ready)
 def test_recovery_missing_registration_refuses_reinstall(self):
  admitted=self.recovery();self.registered=False
  with patch.object(s.time,'sleep'),self.assertRaisesRegex(ValueError,'qualified_registration_absent'):self.owner.ensure(False,admitted)
  self.assertTrue(self.owner.retire());self.assertEqual(self.commands,['query','query'])
 def test_recovery_query_loss_keeps_retirement_unconfirmed(self):
  admitted=self.recovery();self.fail='query'
  with self.assertRaises(ValueError):self.owner.ensure(False,admitted)
  self.assertFalse(self.owner.retire());self.assertEqual(self.commands,['query'])
 def test_recovery_readiness_failure_still_stops_and_never_grants_ready(self):
  admitted=self.recovery();self.listener.return_value=False
  with patch.object(s.time,'monotonic',side_effect=[0,1,30]),patch.object(s.time,'sleep'),self.assertRaisesRegex(ValueError,'readiness_failed'):self.owner.ensure(False,admitted)
  self.assertTrue(self.owner.retire());self.assertFalse(self.owner.ready);self.assertEqual(self.commands.count('stop'),1);self.assertNotIn('install',self.commands)
 def test_recovery_stop_loss_never_claims_clean_preparation(self):
  admitted=self.recovery();self.owner.ensure(False,admitted);self.fail='stop'
  self.assertFalse(self.owner.retire());self.assertFalse(self.owner.retire());self.assertEqual(self.commands.count('stop'),1)
  self.assertFalse(self.owner.service_retirement_confirmed);self.assertTrue(self.owner.forced_cleanup_used)


class BoundMode(unittest.TestCase):
 def test_deleted_recovery_image_cannot_fall_back_to_install(self):
  with tempfile.TemporaryDirectory() as tmp,patch.object(pathlib.Path,'home',return_value=pathlib.Path(tmp)),patch.object(s,'nad1_recovery',side_effect=FileNotFoundError('vanished')),patch.object(s,'nad1_admitted') as admit,patch.object(s.subprocess,'Popen') as launch:
   with self.assertRaises(FileNotFoundError):s.nad1_preparation_inputs({'dependency_mode':'recover_installed'})
   admit.assert_not_called();launch.assert_not_called()
 def test_mode_never_comes_from_operator_arguments_or_missing_schema(self):
  for mode in (None,'install','direct',{},True):
   with self.assertRaises(ValueError):s.nad1_preparation_inputs({'dependency_mode':mode})
 def test_prepare_cannot_silently_adopt_new_unadmitted_image(self):
  with tempfile.TemporaryDirectory() as tmp,patch.object(pathlib.Path,'home',return_value=pathlib.Path(tmp)),patch.object(s,'nad1_recovery') as recover:
   root=pathlib.Path(tmp);p=root/'compatdata/pfx/drive_c'/s.NAD1_DAEMON;p.parent.mkdir(parents=True);p.write_bytes(b'appeared')
   with self.assertRaisesRegex(ValueError,'unadmitted'):s.nad1_preparation_inputs({'dependency_mode':'prepare','application':{'environment':{'root':str(root)}}})
   recover.assert_not_called()
 def test_broken_artifact_cannot_fall_back_to_recovery(self):
  with tempfile.TemporaryDirectory() as tmp,patch.object(pathlib.Path,'home',return_value=pathlib.Path(tmp)),patch.object(s,'nad1_admitted',side_effect=FileNotFoundError('missing referenced source')),patch.object(s,'nad1_recovery') as recover:
   p=pathlib.Path(tmp)/'.local/share/linux-vst-bridge/managed/vendor-applications/native-access-dependency/artifact.json';p.parent.mkdir(parents=True);p.write_bytes(b'{}')
   with self.assertRaises(FileNotFoundError):s.nad1_preparation_inputs({'dependency_mode':'prepare'})
   with self.assertRaisesRegex(ValueError,'state_changed'):s.nad1_preparation_inputs({'dependency_mode':'recover_installed'})
   recover.assert_not_called()

class PreparationSchema(unittest.TestCase):
 def test_closed_saved_mode_is_removed_only_for_shared_renderer_verification(self):
  with tempfile.TemporaryDirectory() as tmp:
   home=pathlib.Path(tmp).resolve();managed=home/'.local/share/linux-vst-bridge/managed';e=s.RENDERER_PRODUCTION
   root=managed/'environments'/e['environment'];appdir=root/'compatdata/pfx/drive_c/Program Files/Native Instruments/Native Access'
   d=managed/'vendor-applications/native-access-dependency/operations'/('a'*32);d.mkdir(parents=True)
   env={'id':e['environment'],'root':str(root),'revision':1,'runner':{'files':[]}}
   app={'schema':1,'id':'native-access','environment':env,'files':{n:{'artifact':{'path':str(appdir/n),'sha256':v['sha256']},'size':v['size']} for n,v in e['files'].items()},'installation':{'path':str(managed/'onboarding'/e['environment']/(e['installation_operation']+'-result.json')),'sha256':e['result_sha256']},'observation_sha256':e['observation_sha256'],'source_seal_sha256':e['source_seal_sha256']}
   identity=hashlib.sha256(json.dumps(app,sort_keys=True,separators=(',',':')).encode()).hexdigest()
   spec={'schema':2,'kind':'native_access_dependency','operation':'a'*32,'application':app,'application_identity':identity,'renderer_policy':'software_rendering','software':{},'software_sha256':'b'*64,'installer_launch':None,'report':str(d/'result.json'),'dependency_mode':'prepare'}
   records={str(root/'environment.json'):env,str(managed/'software.json'):{},str(d/'spec.json'):spec,str(d.parents[1]/'current.json'):{'operation':'a'*32,'application':identity,'policy':'software_rendering'}}
   class ReachedSharedVerifier(Exception):pass
   def verify(normalized):
    self.assertEqual(normalized['schema'],1);self.assertEqual(normalized['kind'],'renderer_application');self.assertNotIn('dependency_mode',normalized)
    raise ReachedSharedVerifier()
   with patch.object(pathlib.Path,'home',return_value=home),patch.object(s,'renderer_read',side_effect=lambda p:records[str(p)]),patch.object(s,'renderer_bound_inputs',side_effect=verify):
    for mode in ('prepare','recover_installed'):
     spec['dependency_mode']=mode
     with self.assertRaises(ReachedSharedVerifier):s.renderer_validate(spec,dependency=True)
    for mutation in ('old_schema','missing','unknown','extra'):
     v=copy.deepcopy(spec)
     if mutation=='old_schema':v['schema']=1
     if mutation=='missing':del v['dependency_mode']
     if mutation=='unknown':v['dependency_mode']='install_anything'
     if mutation=='extra':v['command']='arbitrary'
     with self.assertRaises(ValueError):s.renderer_validate(v,dependency=True)

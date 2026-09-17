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
 def recovery(self):
  admitted=self.existing();self.owner.recovery={'authority':'qualified_bundle_payload_and_retired_installation','daemon':admitted,'prior_failure_preserved':True,'installer_reexecuted':False};return admitted
 def test_recovery_starts_and_retires_without_reinstall_or_direct_execution(self):
  self.owner.ensure(False,self.recovery());self.assertTrue(self.owner.retire())
  self.assertEqual(self.commands,['query','start','query','stop','query']);self.assertTrue(self.owner.ready)
 def test_recovery_missing_registration_refuses_reinstall(self):
  admitted=self.recovery();self.registered=False
  with self.assertRaisesRegex(ValueError,'recovery_registration_missing'):self.owner.ensure(False,admitted)
  self.assertTrue(self.owner.retire());self.assertEqual(self.commands,['query'])
 def test_recovery_query_loss_keeps_retirement_unconfirmed(self):
  admitted=self.recovery();self.fail='query'
  with self.assertRaises(ValueError):self.owner.ensure(False,admitted)
  self.assertFalse(self.owner.retire());self.assertEqual(self.commands,['query'])
 def test_recovery_readiness_failure_still_stops_and_never_grants_ready(self):
  admitted=self.recovery();self.listener.return_value=False
  with patch.object(s.time,'monotonic',side_effect=[0,1,30]),patch.object(s.time,'sleep'),self.assertRaisesRegex(ValueError,'running_not_ready'):self.owner.ensure(False,admitted)
  self.assertTrue(self.owner.retire());self.assertFalse(self.owner.ready);self.assertEqual(self.commands.count('stop'),1);self.assertNotIn('install',self.commands)
 def test_recovery_stop_loss_never_claims_clean_preparation(self):
  self.owner.ensure(False,self.recovery());self.fail='stop'
  self.assertFalse(self.owner.retire());self.assertFalse(self.owner.retire());self.assertEqual(self.commands.count('stop'),1)
  self.assertFalse(self.owner.service_retirement_confirmed);self.assertTrue(self.owner.forced_cleanup_used)


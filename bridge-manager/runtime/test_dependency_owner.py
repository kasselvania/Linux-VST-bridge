"""Closed NAD1 production state/SCM/receipt tests. No proprietary process."""
import copy,hashlib,json,pathlib,struct,tempfile,unittest
from unittest.mock import patch
import session as s
import ownership

class OwnerTests(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup);self.root=pathlib.Path(self.tmp.name).resolve();self.drive=self.root/'compatdata/pfx/drive_c';self.drive.mkdir(parents=True)
  self.installer=self.drive/'Setup.exe';self.daemon=self.drive/'NTKDaemon.exe'
  self.bytes=bytearray(256);self.bytes[:2]=b'MZ';struct.pack_into('<I',self.bytes,60,64);self.bytes[64:68]=b'PE\0\0';self.bytes[68:70]=b'\x64\x86';self.installer.write_bytes(self.bytes)
  self.artifact=ownership.image_identity(self.installer);self.report=self.root/'result.json'
  spec={'operation':'a'*32,'report':str(self.report),'application':{'environment':{'root':str(self.root)}}}
  fixture={'installer':'Setup.exe','daemon':'NTKDaemon.exe','installer_sha256':self.artifact['sha256'],'installer_size':256}
  self.owner=s.Nad1Owner(spec,type('Ledger',(),{'harvest':lambda _:None})(),None,lambda:False,fixture=fixture)
  self.commands=[];self.registered=False;self.service_state=1;self.fail=None
  def command(action):
   self.commands.append(action)
   if action==self.fail:raise ValueError('generated_uncertain_acknowledgment')
   if action=='install':self.daemon.write_bytes(self.bytes);self.registered=True;return None
   if action=='start':self.service_state=4
   return {'registration':'exact' if self.registered else 'absent','state':self.service_state,'image_sha256':self.artifact['sha256']}
  self.owner.command=command
  self.scan={'candidates':[],'private':[],'unavailable':0}
  def scan(*_):
   value=copy.deepcopy(self.scan)
   if self.service_state==4 and not value['candidates']:
    value['candidates']=[{'prefix_relation':'same','exact':True}];value['private']=[{'prefix_relation':'same','exact':True,'linux_pid':10,'start_ticks':20}]
   return value
  self.scan_patch=patch.object(ownership,'census',side_effect=scan);self.scan_patch.start();self.addCleanup(self.scan_patch.stop)
  self.listener=patch.object(s,'nad1_listener_witness',return_value=True).start();self.addCleanup(patch.stopall)
 def existing(self,registered=True,state=1):self.daemon.write_bytes(self.bytes);self.registered=registered;self.service_state=state;return self.artifact
 def test_absent_installs_registers_then_service_only_start(self):
  r=self.owner.ensure(True);self.assertTrue(r['ready_tested']);self.assertEqual(self.commands,['query','install','query','start','query'])
 def test_unregistered_installs_but_never_direct_exec(self):
  self.owner.ensure(True,self.existing(False));self.assertIn('install',self.commands);self.assertIn('start',self.commands)
 def test_stopped_only_starts_service(self):
  self.owner.ensure(True,self.existing());self.assertEqual(self.commands,['query','start','query'])
 def test_ready_no_start_or_install(self):
  self.owner.ensure(True,self.existing(state=4));self.assertEqual(self.commands,['query','query'])
 def test_running_not_ready_does_not_reinstall(self):
  admitted=self.existing(state=4);self.listener.return_value=False
  with patch.object(s.time,'monotonic',side_effect=[0,1,30]),patch.object(s.time,'sleep'),self.assertRaisesRegex(ValueError,'running_not_ready'):self.owner.ensure(True,admitted)
  self.assertNotIn('install',self.commands);self.assertNotIn('start',self.commands)
 def test_foreign_deleted_refuse_before_scm(self):
  admitted=self.existing()
  for relation in ['foreign','deleted']:
   self.scan['candidates']=[{'prefix_relation':relation,'exact':True}]
   with self.assertRaisesRegex(ValueError,'foreign_conflict'):self.owner.ensure(True,admitted)
  self.assertEqual(self.commands,[])
 def test_unresolved_refuses_even_with_absent_files(self):
  self.scan['unavailable']=1
  with self.assertRaisesRegex(ValueError,'identity_unresolved'):self.owner.ensure(True)
  self.assertEqual(self.commands,[])
 def test_installer_ack_loss_not_retried(self):
  self.fail='install'
  with self.assertRaisesRegex(ValueError,'uncertain_acknowledgment'):self.owner.ensure(True)
  self.assertEqual(self.commands.count('install'),1);self.assertNotIn('start',self.commands)
 def test_start_ack_loss_not_retried(self):
  self.fail='start'
  with self.assertRaisesRegex(ValueError,'uncertain_acknowledgment'):self.owner.ensure(True,self.existing())
  self.assertEqual(self.commands.count('start'),1);self.assertNotIn('install',self.commands)
 def test_application_path_cannot_reinstall(self):
  with self.assertRaisesRegex(ValueError,'prepare_required'):self.owner.ensure(False)
  self.assertNotIn('install',self.commands)
 def test_changed_daemon_installer_or_unknown_generation_refuses(self):
  self.existing()
  with self.assertRaisesRegex(ValueError,'unadmitted'):self.owner.ensure(True)
  with self.assertRaisesRegex(ValueError,'generation_changed'):self.owner.ensure(True,dict(self.artifact,sha256='0'*64))
  self.installer.write_bytes(b'changed')
  with self.assertRaises(ValueError):self.owner.ensure(True,self.artifact)
  self.assertEqual(self.commands,[])
 def test_closed_transition_table(self):
  for state in ['running_not_ready','foreign_conflict','unresolved']:self.assertEqual(s.nad1_transition(state),'refuse')
  with self.assertRaises(ValueError):s.nad1_transition('direct_exec')
 def test_immutable_receipt(self):
  s.nad1_publish(self.report,{'state':'first_failure'})
  with self.assertRaises(FileExistsError):s.nad1_publish(self.report,{'state':'success'})
  self.assertEqual(json.loads(self.report.read_bytes()),{'state':'first_failure'})
 def test_scm_generation_protocol_is_closed_and_separate(self):
  op='a'*32;token='b'*64;raw=f'NAD1_SCM_V1 {op} {token} exact 0 4 123 1000 '+self.artifact['sha256']+' 0 0\n'
  v=s.nad1_scm_frame(raw.encode(),op,token);self.assertEqual(v['windows_pid'],123);self.assertNotIn('linux_pid',v)
  for changed in [raw.replace(token,'c'*64),raw+raw,raw.replace('4 123','4 0'),raw.replace(self.artifact['sha256'],'none'),raw.replace(' 4 ',' 99 ')]:
   with self.assertRaises(ValueError):s.nad1_scm_frame(changed.encode(),op,token)
 def test_service_launch_or_exit_zero_without_protocol_refuses(self):
  for raw in [b'',b'0\n',b'SERVICE_RUNNING\n']:
   with self.assertRaises(ValueError):s.nad1_scm_frame(raw,'a'*32,'b'*64)
 def test_admission_never_accepts_arbitrary_fields(self):
  for field in ['service','port','args','command','environment','pid','fixture','test_mode']:
   with patch.object(s.subprocess,'Popen') as launch,patch.object(s.fcntl,'flock') as lock,patch.object(s,'nad1_publish') as write:
    with self.assertRaises(ValueError):s.nad1_application({'schema':1,'kind':'native_access_dependency',field:'untrusted'})
    launch.assert_not_called();lock.assert_not_called();write.assert_not_called()
 def test_no_renderer_launch_without_preparation(self):
  with patch.object(s,'renderer_validate'),patch.object(s,'nad1_prepared',side_effect=ValueError('not_ready')),patch.object(s,'renderer_owned') as run:
   with self.assertRaisesRegex(ValueError,'not_ready'):s.renderer_application({})
   run.assert_not_called()
if __name__=='__main__':unittest.main()

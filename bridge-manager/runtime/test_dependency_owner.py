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
   if action=='stop':self.service_state=1
   return {'registration':'exact' if self.registered else 'absent','state':self.service_state,'image_sha256':self.artifact['sha256'],'owned_endpoint_mask':3 if self.service_state==4 else 0,'windows_pid':123 if self.service_state==4 else 0,'retirement_generation_confirmed':action=='stop'}
  self.owner.command=command
  self.scan={'candidates':[],'private':[],'unavailable':0}
  def scan(*_):
   value=copy.deepcopy(self.scan)
   if self.service_state==4 and not value['candidates']:
    value['candidates']=[{'prefix_relation':'same','exact':True}];value['private']=[{'prefix_relation':'same','exact':True,'linux_pid':10,'start_ticks':20}]
   return value
  self.scan_patch=patch.object(ownership,'census',side_effect=scan);self.scan_patch.start();self.addCleanup(self.scan_patch.stop)
  self.listener=patch.object(s,'nad1_generation_owned',return_value=True).start();self.addCleanup(patch.stopall)
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
  admitted=self.existing(state=4);command=self.owner.command
  self.owner.command=lambda action:dict(command(action),owned_endpoint_mask=0)
  with patch.object(s.time,'monotonic',side_effect=[0,1,30]),patch.object(s.time,'sleep'),self.assertRaisesRegex(ValueError,'running_not_ready'):self.owner.ensure(True,admitted)
  self.assertNotIn('install',self.commands);self.assertNotIn('start',self.commands)
 def test_unowned_or_unavailable_windows_endpoints_cannot_be_ready(self):
  admitted=self.existing(state=4);command=self.owner.command
  for mask in [0,1,2,4]:
   self.owner.command=lambda action:dict(command(action),owned_endpoint_mask=mask)
   with patch.object(s.time,'monotonic',side_effect=[0,1,30]),patch.object(s.time,'sleep'),self.assertRaisesRegex(ValueError,'running_not_ready'):self.owner.ensure(True,admitted)
  self.assertNotIn('install',self.commands)
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
 def test_retirement_after_success_is_idempotent_and_separate_from_cleanup(self):
  self.owner.ensure(True)
  self.assertTrue(self.owner.retire());self.assertTrue(self.owner.retire())
  self.assertEqual(self.commands.count('stop'),1)
  self.assertTrue(self.owner.value()['service_retirement_confirmed'])
  self.assertFalse(self.owner.value()['process_cleanup_confirmed'])
  self.assertFalse(self.owner.value()['forced_cleanup_used'])
 def test_failure_and_cancel_still_stop_exact_service_once(self):
  for failure in ['not_ready','cancel','start_ack']:
   with self.subTest(failure=failure):
    self.commands.clear();self.owner.retirement_attempted=False;self.owner.retiring=False
    self.owner.service_retirement_confirmed=False;self.owner.cancelled=lambda:False
    admitted=self.existing();self.fail='start' if failure=='start_ack' else None
    if failure=='cancel':self.owner.cancelled=lambda:True
    if failure=='not_ready':self.listener.return_value=False
    with patch.object(s.time,'monotonic',side_effect=[0,1,30]),patch.object(s.time,'sleep'),self.assertRaises(ValueError):self.owner.ensure(True,admitted)
    self.fail=None
    self.assertTrue(self.owner.retire());self.assertEqual(self.commands.count('stop'),1)
 def test_stop_acknowledgment_loss_never_promotes_forced_cleanup(self):
  self.owner.ensure(True);self.fail='stop'
  self.assertFalse(self.owner.retire());self.assertFalse(self.owner.retire())
  self.owner.process_cleanup_confirmed=True
  self.assertEqual(self.commands.count('stop'),1)
  self.assertFalse(self.owner.value()['service_retirement_confirmed'])
  self.assertTrue(self.owner.value()['forced_cleanup_used'])
  self.assertEqual(self.owner.retirement_error,'generated_uncertain_acknowledgment')
 def test_query_state_or_linux_survivor_cannot_claim_service_retirement(self):
  self.owner.ensure(True);command=self.owner.command
  self.owner.command=lambda a:dict(command(a),windows_pid=123) if a=='query' else command(a)
  with patch.object(s.time,'monotonic',side_effect=[0,1,30]),patch.object(s.time,'sleep'):
   self.assertFalse(self.owner.retire())
  self.assertTrue(self.owner.forced_cleanup_used)
 def test_foreign_refusal_never_stops_service(self):
  self.scan['candidates']=[{'prefix_relation':'foreign'}]
  with self.assertRaises(ValueError):self.owner.ensure(True)
  self.assertFalse(self.owner.retire());self.assertFalse(self.owner.service_stop_requested)
  self.assertEqual(self.commands,[])
 def test_first_query_loss_with_admitted_daemon_never_confirms_retirement(self):
  admitted=self.existing();self.fail='query'
  with self.assertRaisesRegex(ValueError,'uncertain_acknowledgment'):self.owner.ensure(True,admitted)
  self.assertFalse(self.owner.retire());self.assertFalse(self.owner.retire())
  self.assertEqual(self.owner.service_possibility,'may_exist')
  self.assertEqual(self.commands,['query']);self.assertFalse(self.owner.service_stop_requested)
 def test_same_prefix_unowned_refusal_has_no_signal_or_retirement_authority(self):
  admitted=self.existing(state=4);self.listener.return_value=False
  with self.assertRaisesRegex(ValueError,'same_prefix_unowned'):self.owner.ensure(True,admitted)
  self.assertFalse(self.owner.retire());self.assertFalse(self.owner.service_stop_requested)
  self.assertEqual(self.commands,[]);self.assertEqual(self.owner.service_possibility,'may_exist')
 def test_foreign_and_deleted_refusal_cannot_publish_clean_retirement(self):
  for relation in ('foreign','deleted'):
   with self.subTest(relation=relation):
    self.scan['candidates']=[{'prefix_relation':relation}]
    with self.assertRaisesRegex(ValueError,'foreign_conflict'):self.owner.ensure(True)
    self.assertFalse(self.owner.retire());self.assertFalse(self.owner.service_stop_requested)
    self.assertEqual(self.commands,[])
 def test_exact_absent_query_retains_absence_without_stop(self):
  with self.assertRaisesRegex(ValueError,'prepare_required'):self.owner.ensure(False)
  self.assertTrue(self.owner.retire());self.assertEqual(self.commands,['query'])
  self.assertEqual(self.owner.value()['service_possibility'],'proved_absent')
  self.assertFalse(self.owner.service_stop_requested)
 def test_unknown_without_query_is_not_absence(self):
  self.assertFalse(self.owner.retire());self.assertEqual(self.commands,[])
  self.assertFalse(self.owner.service_retirement_confirmed)
 def test_install_ack_loss_authorizes_exact_retirement_once(self):
  self.fail='install'
  with self.assertRaisesRegex(ValueError,'uncertain_acknowledgment'):self.owner.ensure(True)
  self.assertEqual(self.owner.service_possibility,'may_exist')
  self.assertTrue(self.owner.retire());self.assertTrue(self.owner.retire())
  self.assertEqual(self.commands,['query','install','stop','query'])
 def test_only_exact_service_not_found_frame_proves_absence(self):
  op='a'*32;token='b'*64
  raw=f'NAD1_SCM_V1 {op} {token} absent 1060 0 0 0 none 0 0 0\n'
  self.assertEqual(s.nad1_scm_frame(raw.encode(),op,token)['registration'],'absent')
  for error in ('0','5','1062'):
   with self.assertRaises(ValueError):s.nad1_scm_frame(raw.replace('1060',error).encode(),op,token)
 def test_normal_operation_has_no_wall_clock_expiry(self):
  import inspect
  source=inspect.getsource(s.renderer_run)
  self.assertNotIn('application_time_bound',source)
  self.assertNotIn('monotonic()-began',source)
  adapter=pathlib.Path(__file__).resolve().parents[2]/'tools/is2/nad1_service.h'
  if adapter.exists():
   anchor=adapter.read_text().split('// Normal use has no wall-clock deadline.',1)[1]
   self.assertNotIn('GetTickCount',anchor);self.assertNotIn('deadline',anchor)
   self.assertIn('SERVICE_STOPPED',anchor);self.assertIn('WaitForSingleObject(anchor,0)',anchor)
 def test_closed_transition_table(self):
  for state in ['running_not_ready','foreign_conflict','unresolved']:self.assertEqual(s.nad1_transition(state),'refuse')
  with self.assertRaises(ValueError):s.nad1_transition('direct_exec')
 def test_immutable_receipt(self):
  s.nad1_publish(self.report,{'state':'first_failure'})
  with self.assertRaises(FileExistsError):s.nad1_publish(self.report,{'state':'success'})
  self.assertEqual(json.loads(self.report.read_bytes()),{'state':'first_failure'})
 def test_scm_generation_protocol_is_closed_and_separate(self):
  op='a'*32;token='b'*64;raw=f'NAD1_SCM_V1 {op} {token} exact 0 4 123 1000 '+self.artifact['sha256']+' 0 0 3\n'
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

class ListenerCustodyTests(unittest.TestCase):
 def test_exact_linux_generation_and_cgroup_are_independent_of_windows_pid(self):
  candidate={'linux_pid':123,'start_ticks':10};scope=type('Scope',(),{'members':lambda _: [{'pid':123,'start_ticks':10}]})()
  with patch.object(ownership,'bounded',return_value=b'123 (NTKDaemon.exe) S '+b'0 '*18+b'10\n'):
   self.assertTrue(s.nad1_generation_owned(candidate,scope))
   self.assertFalse(s.nad1_generation_owned(dict(candidate,start_ticks=11),scope))
   scope.members=lambda:[];self.assertFalse(s.nad1_generation_owned(candidate,scope))
 def test_windows_endpoint_masks_are_not_helper_success(self):
  op='a'*32;token='b'*64
  for mask in range(5):
   frame=f'NAD1_SCM_V1 {op} {token} exact 0 4 123 1000 '+('c'*64)+f' 0 0 {mask}\n'
   self.assertEqual(s.nad1_scm_frame(frame.encode(),op,token)['owned_endpoint_mask'],mask)
  with self.assertRaises(ValueError):s.nad1_scm_frame(frame.replace(' 4\n',' 5\n').encode(),op,token)
 def test_fixed_service_request_rejects_injection_before_process_creation(self):
  owner=s.Nad1Owner({'operation':'a'*32,'report':'/unused/result.json','application':{'environment':{'root':'/unused'}}},None,None,lambda:False)
  with patch.object(s.subprocess,'Popen') as launch:
   for action in ['start NTKDaemon','direct','install /other','service=other']:
    with self.assertRaisesRegex(ValueError,'dependency_action'):owner.command(action)
   launch.assert_not_called()


class ArtifactRecovery(unittest.TestCase):
 def test_install_identity_survives_readiness_failure_without_granting_readiness(self):
  with tempfile.TemporaryDirectory() as tmp,patch.object(pathlib.Path,'home',return_value=pathlib.Path(tmp)):
   op='a'*32;d=pathlib.Path(tmp)/'.local/share/linux-vst-bridge/managed/vendor-applications/native-access-dependency/operations'/op;d.mkdir(parents=True)
   spec={'operation':op,'report':str(d/'result.json'),'application_identity':'b'*64,'software_sha256':'c'*64,'application':{'exact':True},'software':{'exact':True},'kind':'native_access_dependency'}
   s.installer_atomic(d/'spec.json',spec)
   s.nad1_publish(d/(op+'-installer-root.private.json'),{'frame':['NAD1_INSTALL_ROOT_V1',op,'d'*64,s.NAD1_INSTALLER_SHA,'35769456','10','20']})
   daemon={'sha256':'e'*64,'size':256,'architecture':'x64'}
   s.nad1_retain_artifact(spec,daemon)
   self.assertEqual(s.nad1_admitted(spec),daemon)
   with self.assertRaises(FileNotFoundError):s.nad1_prepared(spec)
   for key in ['software_sha256','application_identity']:
    with self.assertRaises(ValueError):s.nad1_admitted(dict(spec,**{key:'f'*64}))
   (d/(op+'-installer-root.private.json')).write_bytes(b'changed')
   with self.assertRaises(RuntimeError):s.nad1_admitted(spec)

if __name__=='__main__':unittest.main()

@unittest.skipUnless(__import__('sys').platform.startswith('linux'),'Linux production loop and subreaper')
class IntegratedApplicationTests(unittest.TestCase):
 def test_all_terminal_paths_retire_dependency_and_preserve_first_error(self):
  import os,signal,subprocess,sys
  from test_installer import Scope
  for case in ['normal','cancel','readiness_failure','application_failure','anchor_failure','stop_failure','launch_failure']:
   with self.subTest(case=case),tempfile.TemporaryDirectory() as tmp:
    root=pathlib.Path(tmp).resolve();(root/'compatdata/pfx').mkdir(parents=True);(root/'compatdata/pfx/system.reg').touch()
    image=root/'application.exe';image.write_bytes(b'MZfixture');artifact={'path':str(image),'sha256':hashlib.sha256(image.read_bytes()).hexdigest()}
    op='e'*32;spec={'operation':op,'report':str(root/'result.json'),'application_identity':'b'*64,'software_sha256':'c'*64,'renderer_policy':'software_rendering','installer_launch':{'path':'adapter'},'application':{'environment':{'root':str(root),'runner':{'entry_point':'fixture','proton':'fixture'}},'files':{'Native Access.exe':{'artifact':artifact,'size':image.stat().st_size}}}}
    events=[];children=[];real=subprocess.Popen;prior=[signal.getsignal(x) for x in (signal.SIGTERM,signal.SIGINT)]
    class Dependency:
     anchor=type('Exited',(),{'returncode':1})() if case=='anchor_failure' else None
     process_cleanup_confirmed=False;forced_cleanup_used=False
     def __init__(self,*_,**__):pass
     def ensure(self,*_):
      events.append('ready')
      if case=='readiness_failure':raise ValueError('dependency_running_not_ready')
     def retire(self):events.append('service_stop');return case!='stop_failure'
     def value(self):return {'service_retirement_confirmed':case!='stop_failure','process_cleanup_confirmed':self.process_cleanup_confirmed}
    def launch(*args,**kwargs):
     self.assertEqual(events,['ready']);events.append('application_launch')
     if case=='launch_failure':raise OSError('generated refusal')
     request=(root/(op+'-launch.private')).read_bytes().decode('utf-16le').splitlines()
     frame='IS2_ROOT_V1 '+' '.join(request[1:6])+' 100 1000 50 500\n'
     code=7 if case in ('application_failure','stop_failure') else 0
     child=real([sys.executable,'-c','import sys,time;sys.stderr.write('+repr(frame)+');sys.stderr.flush();time.sleep(.2);sys.exit('+str(code)+')'],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
     children.append(child)
     if case=='cancel':signal.getsignal(signal.SIGTERM)(None,None)
     return child
    try:
     with patch.object(s,'CompanionCgroup',return_value=Scope()),patch.object(s,'environment',return_value={}),patch.object(s,'Nad1Owner',Dependency),patch.object(s,'renderer_retire_image',side_effect=lambda *_:events.append('application_retire')),patch.object(s.subprocess,'Popen',side_effect=launch):
      s.renderer_owned(spec,dependency={'daemon':{}})
     r=json.loads((root/'result.json').read_bytes())
     self.assertEqual(events.count('service_stop'),1);self.assertTrue(r['cleanup_confirmed']);self.assertTrue(r['dependency']['process_cleanup_confirmed'])
     if case in ('readiness_failure','launch_failure'):
      self.assertNotIn('application_retire',events)
     else:self.assertLess(events.index('application_retire'),events.index('service_stop'))
     if case=='readiness_failure':self.assertEqual(events,['ready','service_stop'])
     if case=='stop_failure':self.assertEqual(r['error'],'application_outer_nonzero');self.assertFalse(r['dependency']['service_retirement_confirmed'])
     if case=='normal':self.assertEqual(r['state'],'completed')
     elif case=='cancel':self.assertEqual(r['state'],'cancelled')
     else:self.assertEqual(r['state'],'failed')
    finally:
     for sig,value in zip((signal.SIGTERM,signal.SIGINT),prior):signal.signal(sig,value)
     for child in children:
      if child.poll() is None:child.kill();child.wait()
     s.ctypes.CDLL(None).prctl(36,0,0,0,0)

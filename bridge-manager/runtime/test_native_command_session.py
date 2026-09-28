"""Native shared-runtime custody: real Linux children, plus admission negatives."""
import hashlib,json,os,pathlib,signal,socket,subprocess,sys,tempfile,threading,time,unittest
from unittest.mock import Mock,patch
from types import SimpleNamespace
import session as s
import ownership as o

class SelectionTests(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
  self.root=pathlib.Path(self.tmp.name).resolve();self.root.chmod(0o700)
  self.envroot=self.root/'environments'/('a'*32);self.envroot.mkdir(parents=True)
  self.runner={'proton':str(self.root/'proton'),'entry_point':str(self.root/'runtime/_v2-entry-point'),'files':[]}
  self.env={'id':'a'*32,'revision':4,'root':str(self.envroot),'runner':self.runner}
  self.spec={'session':'b'*32,'shared_runtime':True,'runner_key':'f'*64,'registration':{'environment':self.env}}
  base=self.root/'runtime/pressure-vessel/bin';base.mkdir(parents=True)
  self.component={'schema':1,'kind':'native_proton_command_session'}
  for key,name in [('client_sha256','steam-runtime-launch-client'),('service_sha256','../libexec/steam-runtime-tools-0/x86_64-linux-gnu-srt-launcher-service')]:
   p=base/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(name.encode());self.component[key]=hashlib.sha256(p.read_bytes()).hexdigest()
 def declare(self):
  p=self.root/s.NativeProtonSession.COMPONENT;p.write_text(json.dumps(self.component));p.chmod(0o600)
  self.runner['files']=[{'path':str(p),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}]
 def test_ambient_component_never_selects_execution(self):
  self.declare();self.runner['files']=[];self.assertIsNone(s.NativeProtonSession.selected(self.spec))
 def test_declared_component_and_each_tool_must_match(self):
  self.declare();self.assertIsNotNone(s.NativeProtonSession.selected(self.spec))
  (self.root/'runtime/pressure-vessel/bin/steam-runtime-launch-client').write_bytes(b'changed')
  with self.assertRaisesRegex(RuntimeError,'changed'):s.NativeProtonSession.selected(self.spec)
 def test_unsupported_component_refuses(self):
  self.component['kind']='arbitrary_commands';self.declare()
  with self.assertRaisesRegex(RuntimeError,'unsupported'):s.NativeProtonSession.selected(self.spec)
 def test_duplicate_malformed_and_changed_component_refuse(self):
  self.declare();self.runner['files'].append(dict(self.runner['files'][0]))
  with self.assertRaisesRegex(RuntimeError,'ambiguous'):s.NativeProtonSession.selected(self.spec)
  self.runner['files'].pop();p=pathlib.Path(self.runner['files'][0]['path']);p.write_bytes(b'changed')
  with self.assertRaisesRegex(RuntimeError,'component changed'):s.NativeProtonSession.selected(self.spec)
  self.component['schema']=2;self.declare()
  with self.assertRaisesRegex(RuntimeError,'unsupported'):s.NativeProtonSession.selected(self.spec)
  self.component['schema']=1;self.component['extra']='unreviewed';self.declare()
  with self.assertRaisesRegex(RuntimeError,'unsupported'):s.NativeProtonSession.selected(self.spec)
 def test_changed_launcher_service_and_missing_canonical_key_refuse(self):
  self.declare();service=self.root/'runtime/pressure-vessel/libexec/steam-runtime-tools-0/x86_64-linux-gnu-srt-launcher-service'
  service.write_bytes(b'changed')
  with self.assertRaisesRegex(RuntimeError,'changed'):s.NativeProtonSession.selected(self.spec)
  self.spec.pop('runner_key')
  with self.assertRaisesRegex(RuntimeError,'canonical runner key absent'):s.NativeProtonSession.selected(self.spec)
 def test_nonshared_paths_keep_their_existing_owner(self):
  self.declare();self.spec['shared_runtime']=False;self.assertIsNone(s.NativeProtonSession.selected(self.spec))
 def test_missing_or_dead_keeper_never_falls_back_to_new_container(self):
  self.declare();n=s.NativeProtonSession.selected(self.spec)
  (self.envroot/'compatdata/pfx/drive_c/bridge/sessions').mkdir(parents=True)
  with self.assertRaisesRegex(RuntimeError,'one live exact keeper'):n.find_keeper()
 def test_endpoint_rejects_nonidentity_path(self):
  with self.assertRaisesRegex(RuntimeError,'keeper identity'):s.NativeProtonSession.endpoint_for('../arbitrary')
 def test_private_command_endpoint_preserves_graphical_denial(self):
  self.declare();n=s.NativeProtonSession.selected(self.spec)
  endpoint=self.root/'ipc'/'keeper'/'s';env={'DBUS_SESSION_BUS_ADDRESS':'unix:path=/denied','PATH':'/usr/bin:/bin'}
  with patch.object(n,'endpoint_for',return_value=endpoint):
   command,actual=n.keeper_launch(['entry','--verb=run','--','proton','runinprefix'],env)
  self.assertEqual(actual,env)
  self.assertIn('--socket='+str(endpoint),command)
  self.assertNotIn('--session',command);self.assertNotIn('--exec-fallback',command)
  n.retire_endpoint()
 def test_endpoint_extent_and_private_owner_posture_refuse(self):
  self.declare();n=s.NativeProtonSession.selected(self.spec)
  long_endpoint=self.root/('x'*96)/'s'
  with patch.object(n,'endpoint_for',return_value=long_endpoint):
   with self.assertRaisesRegex(RuntimeError,'socket path extent'):n.keeper_launch(['entry','--verb=run','--','proton'],{})
  directory=self.root/'public'/('d'*32);directory.parent.mkdir(mode=0o700);directory.parent.chmod(0o777)
  with patch.object(n,'endpoint_for',return_value=directory/'s'):
   try:
    with self.assertRaises(RuntimeError):n.keeper_launch(['entry','--verb=run','--','proton'],{})
   finally:directory.parent.chmod(0o700)
 def test_keeper_requires_live_owner_ready_report_and_matching_socket(self):
  self.declare();n=s.NativeProtonSession.selected(self.spec)
  directory=self.envroot/'compatdata/pfx/drive_c/bridge/sessions'/('d'*32);directory.mkdir(parents=True);directory.chmod(0o700)
  report=self.root/'runtime/results'/('environment-'+directory.name+'.json');report.parent.mkdir(parents=True);report.write_text('{"ready":true}');report.chmod(0o600)
  owner={'keeper':True,'session':directory.name,'runner_key':self.spec['runner_key'],'registration':self.spec['registration'],'report':str(report)}
  (directory/'owner.json').write_text(json.dumps(owner));(directory/'owner.json').chmod(0o600)
  (directory/'environment.ready').write_text(directory.name+'\n')
  descriptor={'schema':1,'keeper':directory.name,'owner_pid':123,'owner_start':456,'runner':n.runner_key(),'context':None,'socket_identity':[3,4]}
  (directory/'command-session.json').write_text(json.dumps(descriptor));(directory/'command-session.json').chmod(0o600)
  endpoint=self.root/'socket'
  with patch.object(n,'endpoint_for',return_value=endpoint),patch.object(s,'ProcessTracker') as tracker,patch.object(s,'socket_identity',return_value=(3,4)):
   tracker.return_value.identity.return_value=(456,1)
   n.find_keeper();self.assertEqual(n.endpoint,endpoint)
   with patch.object(s,'socket_identity',return_value=(3,5)):
    with self.assertRaisesRegex(RuntimeError,'endpoint changed'):n.find_keeper()
   with patch.object(tracker.return_value,'identity',return_value=(457,1)):
    with self.assertRaisesRegex(RuntimeError,'one live exact keeper'):n.find_keeper()
   report.write_text('{"ready":false}')
   with self.assertRaisesRegex(RuntimeError,'one live exact keeper'):n.find_keeper()
 def test_keeper_descriptor_owner_context_and_readiness_refuse_drift(self):
  self.declare();n=s.NativeProtonSession.selected(self.spec)
  directory=self.envroot/'compatdata/pfx/drive_c/bridge/sessions'/('d'*32);directory.mkdir(parents=True);directory.chmod(0o700)
  report=self.root/'runtime/results'/('environment-'+directory.name+'.json');report.parent.mkdir(parents=True);report.write_text('{"ready":true}');report.chmod(0o600)
  owner={'keeper':True,'session':directory.name,'runner_key':self.spec['runner_key'],'registration':self.spec['registration'],'report':str(report)}
  owner_path=directory/'owner.json';owner_path.write_text(json.dumps(owner));owner_path.chmod(0o600)
  ready=directory/'environment.ready';ready.write_text(directory.name+'\n')
  descriptor={'schema':1,'keeper':directory.name,'owner_pid':123,'owner_start':456,'runner':n.runner_key(),'context':None,'socket_identity':[3,4]}
  descriptor_path=directory/'command-session.json';descriptor_path.write_text(json.dumps(descriptor));descriptor_path.chmod(0o600)
  endpoint=self.root/'socket'
  with patch.object(n,'endpoint_for',return_value=endpoint),patch.object(s,'ProcessTracker') as tracker,patch.object(s,'socket_identity',return_value=(3,4)):
   tracker.return_value.identity.return_value=(456,1)
   n.find_keeper()
   for field,value in [('runner','0'*64),('context',{'display':':9'}),('schema',2)]:
    changed=dict(descriptor);changed[field]=value;descriptor_path.write_text(json.dumps(changed))
    with self.assertRaises(RuntimeError):n.find_keeper()
   descriptor_path.write_text(json.dumps(descriptor))
   changed=dict(owner);changed['runner_key']='0'*64;owner_path.write_text(json.dumps(changed))
   with self.assertRaisesRegex(RuntimeError,'environment binding'):n.find_keeper()
   changed=dict(owner);changed['report']=str(self.root/'foreign.json');owner_path.write_text(json.dumps(changed))
   with self.assertRaisesRegex(RuntimeError,'environment binding'):n.find_keeper()
   changed=dict(owner);changed['graphical_session']={'display':':9'};owner_path.write_text(json.dumps(changed))
   with self.assertRaisesRegex(RuntimeError,'environment binding'):n.find_keeper()
   owner_path.write_text(json.dumps(owner))
   ready.write_text('0'*32+'\n')
   with self.assertRaisesRegex(RuntimeError,'readiness'):n.find_keeper()
   ready.write_text(directory.name+'\n')
   n.find_keeper()
 def test_binding_rejects_bad_nonce_identity_group_and_dead_launcher(self):
  nonce='f'*64
  for label,hello,identity,group,exit_code in (
      ('nonce',{'nonce':'0'*64,'pid':123,'start':456},(456,1),123,None),
      ('identity',{'nonce':nonce,'pid':123,'start':456},(457,1),123,None),
      ('group',{'nonce':nonce,'pid':123,'start':456},(456,1),999,None),
      ('launcher',{'nonce':nonce,'pid':123,'start':456},(456,1),123,1)):
   parent,child=socket.socketpair()
   runtime=object.__new__(s.NativeProtonSession);runtime.control=parent;runtime.control.setblocking(False);runtime.nonce=nonce;runtime.remote_identity=None
   tracker=Mock();tracker.identity.return_value=identity;tracker.owned=set()
   root=SimpleNamespace(poll=lambda:exit_code)
   child.sendall((json.dumps(hello)+'\n').encode())
   try:
    with self.subTest(label=label),patch.object(s.os,'getpgid',return_value=group):
     with self.assertRaisesRegex(RuntimeError,'native command'):runtime.bind(root,tracker,lambda _:None)
     self.assertIsNone(runtime.remote_identity)
   finally:runtime.close();child.close()
 def test_multiple_matching_keepers_and_wrong_owner_environment_refuse(self):
  self.declare();n=s.NativeProtonSession.selected(self.spec)
  sessions=self.envroot/'compatdata/pfx/drive_c/bridge/sessions'
  owners=[]
  for sid in ('d'*32,'e'*32):
   directory=sessions/sid;directory.mkdir(parents=True);directory.chmod(0o700)
   report=self.root/'runtime/results'/('environment-'+sid+'.json');report.parent.mkdir(parents=True,exist_ok=True);report.write_text('{"ready":true}');report.chmod(0o600)
   owner={'keeper':True,'session':sid,'runner_key':self.spec['runner_key'],'registration':self.spec['registration'],'report':str(report)}
   owner_path=directory/'owner.json';owner_path.write_text(json.dumps(owner));owner_path.chmod(0o600);owners.append((owner_path,owner))
   (directory/'environment.ready').write_text(sid+'\n')
   descriptor={'schema':1,'keeper':sid,'owner_pid':123,'owner_start':456,'runner':n.runner_key(),'context':None,'socket_identity':[3,4]}
   p=directory/'command-session.json';p.write_text(json.dumps(descriptor));p.chmod(0o600)
  with patch.object(n,'endpoint_for',return_value=self.root/'socket'),patch.object(s,'ProcessTracker') as tracker,patch.object(s,'socket_identity',return_value=(3,4)):
   tracker.return_value.identity.return_value=(456,1)
   with self.assertRaisesRegex(RuntimeError,'one live exact keeper'):n.find_keeper()
   owner_path,owner=owners[1];changed=json.loads(json.dumps(owner));changed['registration']['environment']['id']='0'*32;owner_path.write_text(json.dumps(changed))
   with self.assertRaisesRegex(RuntimeError,'environment binding'):n.find_keeper()
 def test_endpoint_and_descriptor_retire_only_at_exact_identity(self):
  self.declare();n=s.NativeProtonSession.selected(self.spec)
  endpoint=self.root/'command-session'/'s';endpoint.parent.mkdir(mode=0o700)
  listener=socket.socket(socket.AF_UNIX);listener.bind(str(endpoint));n.endpoint=endpoint;n.socket_identity=s.socket_identity(endpoint,'test endpoint')
  descriptor=self.root/'command-session.json';n.descriptor=descriptor;n.descriptor_value={'schema':1,'keeper':self.spec['session']}
  descriptor.write_text(json.dumps(n.descriptor_value));descriptor.chmod(0o600)
  try:
   descriptor.write_text('{"changed":true}')
   with self.assertRaisesRegex(RuntimeError,'descriptor changed'):n.retire_endpoint()
   self.assertTrue(endpoint.exists())
   descriptor.write_text(json.dumps(n.descriptor_value))
   n.socket_identity=(0,0)
   with self.assertRaisesRegex(RuntimeError,'endpoint changed'):n.retire_endpoint()
   self.assertTrue(endpoint.exists());self.assertTrue(descriptor.exists())
   n.socket_identity=s.socket_identity(endpoint,'test endpoint')
   n.retire_endpoint();self.assertFalse(endpoint.exists());self.assertFalse(descriptor.exists())
  finally:listener.close()
 def test_published_socket_identity_round_trips_through_json_retirement(self):
  self.declare();directory=self.envroot/'compatdata/pfx/drive_c/bridge/sessions'/self.spec['session'];directory.mkdir(parents=True);directory.chmod(0o700)
  self.spec['directory']=str(directory);n=s.NativeProtonSession.selected(self.spec)
  endpoint=self.root/'command-session'/'s';endpoint.parent.mkdir(mode=0o700)
  listener=socket.socket(socket.AF_UNIX);listener.bind(str(endpoint));n.endpoint=endpoint
  try:
   previous_umask=os.umask(0o077)
   try:
    with patch.object(s,'ProcessTracker') as tracker:
     tracker.return_value.identity.return_value=(456,1)
     self.assertTrue(n.publish())
   finally:os.umask(previous_umask)
   value=json.loads((directory/'command-session.json').read_text())
   self.assertEqual(value['socket_identity'],list(n.socket_identity))
   n.retire_endpoint();self.assertFalse(endpoint.exists());self.assertFalse((directory/'command-session.json').exists())
  finally:listener.close()

@unittest.skipUnless(sys.platform=='linux','Linux kernel process custody')
class KernelCustodyTests(unittest.TestCase):
 def helper(self,target):
  parent,child=socket.socketpair();nonce='c'*64
  p=subprocess.Popen([sys.executable,str(pathlib.Path(s.__file__).resolve()),'--native-command-child',str(child.fileno()),nonce,'--',*target],
    pass_fds=(child.fileno(),),start_new_session=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
  child.close();return p,parent,nonce
 def test_disconnect_before_acknowledgement_never_executes_target(self):
  with tempfile.TemporaryDirectory() as tmp:
   marker=pathlib.Path(tmp)/'executed'
   p,channel,nonce=self.helper(['/usr/bin/touch',str(marker)])
   try:
    channel.settimeout(2);hello=json.loads(channel.recv(512));self.assertEqual(hello['nonce'],nonce)
    channel.close();self.assertNotEqual(p.wait(timeout=3),0);self.assertFalse(marker.exists())
   finally:
    channel.close()
    if p.poll() is None:p.kill();p.wait()
    p.stdout.close();p.stderr.close()
 def test_bound_remote_group_is_retired_after_local_client_dies_and_sibling_survives(self):
  with tempfile.TemporaryDirectory() as tmp:
   marker=pathlib.Path(tmp)/'ready'
   code='import signal,time,pathlib;signal.signal(signal.SIGTERM,signal.SIG_IGN);pathlib.Path('+repr(str(marker))+').touch();time.sleep(30)'
   remote,channel,nonce=self.helper([sys.executable,'-c',code])
   client=subprocess.Popen(['/bin/sleep','30'],start_new_session=True)
   sibling=subprocess.Popen(['/bin/sleep','30'],start_new_session=True)
   runtime=object.__new__(s.NativeProtonSession);runtime.control=channel;runtime.control.setblocking(False);runtime.nonce=nonce;runtime.remote_identity=None
   tracker=o.ProcessTracker(client.pid);reaper=None
   try:
    runtime.bind(client,tracker,lambda _:None)
    self.assertEqual(runtime.remote_identity[0],remote.pid)
    deadline=time.monotonic()+3
    while not marker.exists() and time.monotonic()<deadline:time.sleep(.01)
    self.assertTrue(marker.exists());owned=tracker.update()
    client.kill();client.wait();reaper=threading.Thread(target=remote.wait);reaper.start()
    with patch.object(o,'CLEANUP_SECONDS',3.3):result=o.cleanup_process(client,list(owned))
    reaper.join(1);self.assertFalse(reaper.is_alive());self.assertEqual(remote.returncode,-signal.SIGKILL)
    self.assertEqual(result,{'owned_descendants_zero':True,'process_group_empty':True});self.assertIsNone(sibling.poll())
   finally:
    runtime.close()
    for p in (client,remote,sibling):
     if p.poll() is None:p.kill()
     p.wait()
    if reaper:reaper.join(1)
    remote.stdout.close();remote.stderr.close()
 def test_recycled_group_identity_is_not_signalled(self):
  from types import SimpleNamespace
  root=SimpleNamespace(lvb_remote_group=(123,456))
  census=[{'pid':123,'start_ticks':789,'pgrp':123}]
  with patch.object(o,'process_identities',return_value=census),patch.object(o.os,'killpg') as kill:
   o.signal_remote_group(root,[(123,456)],signal.SIGKILL);kill.assert_not_called()
 def test_untracked_remote_group_member_prevents_positive_cleanup(self):
  from types import SimpleNamespace
  root=SimpleNamespace(pid=98,lvb_remote_group=(123,456),poll=lambda:0,wait=lambda timeout:0)
  census=[{'pid':124,'start_ticks':500,'pgrp':123,'session':123}]
  with patch.object(o,'process_identities',return_value=census),patch.object(o.os,'killpg') as kill:
   with self.assertRaisesRegex(RuntimeError,'survived cleanup'):o.cleanup_process(root,[])
   kill.assert_not_called()

if __name__=='__main__':unittest.main()

"""Operation-container authority, with no Wine or proprietary application."""
import pathlib,tempfile,types,unittest
from unittest.mock import Mock,patch
import session as s

class RuntimeTests(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
  root=pathlib.Path(self.tmp.name).resolve()
  self.owner=types.SimpleNamespace(root=root,directory=root,op='a'*32,retiring=False,
   cancelled=lambda:False,ledger=Mock(),spec={'installer_launch':{'path':'/fixed/adapter.exe'},
   'application':{'environment':{'root':str(root),'runner':{'entry_point':'/fixed/runtime/_v2-entry-point',
   'proton':'/fixed/proton','files':[]}}}})
  self.runtime=s.Nad1Runtime(self.owner);self.addCleanup(self.runtime.close)
 def test_missing_or_duplicate_client_identity_refuses(self):
  artifact={'path':str(self.runtime.client),'sha256':'a'*64}
  for files in ([],[artifact,artifact]):
   self.runtime.runner['files']=files
   with self.assertRaisesRegex(ValueError,'client_identity'),patch.object(s,'verify') as verify:
    self.runtime.verify_tools()
   verify.assert_not_called()
 def test_client_and_service_bytes_are_verified(self):
  artifact={'path':str(self.runtime.client),'sha256':'a'*64};self.runtime.runner['files']=[artifact]
  with patch.object(s,'verify') as verify:self.runtime.verify_tools()
  self.assertEqual(verify.call_args_list[0].args[0],artifact)
  self.assertEqual(verify.call_args_list[1].args[0],{'path':str(self.runtime.service),'sha256':s.Nad1Runtime.SERVICE_SHA})
  with patch.object(s,'verify',side_effect=RuntimeError('changed')):
   with self.assertRaises(RuntimeError):self.runtime.verify_tools()
 def test_commands_reuse_container_and_only_forward_closed_diagnostics(self):
  r=self.runtime;r.socket=pathlib.Path('/fixed/private/socket');r.child=Mock(returncode=None)
  with patch.object(r,'start') as start,patch.object(r,'drain'),patch.object(r,'verify_tools'):
   a=r.argv(self.owner.root/'one.private',self.owner.root,{'EVIL':'value'})
   b=r.argv(self.owner.root/'two.private',self.owner.root/'home',{})
  start.assert_not_called();self.assertEqual(a[0],str(r.client));self.assertEqual(a[1],b[1])
  self.assertNotIn('EVIL',' '.join(a));self.assertNotIn('--clear-env',a)
  self.assertEqual([x for x in a if x.startswith('--pass-env=')],['--pass-env='+x for x in r.DEBUG_KEYS])
  self.assertEqual(a[-4:-1],['/fixed/proton','runinprefix','/fixed/adapter.exe'])
  self.assertIn('--directory='+str(self.owner.root/'home'),b)
 def test_dead_or_retired_container_never_falls_back_to_another_container(self):
  r=self.runtime;r.child=Mock(returncode=1)
  with patch.object(r,'drain'),patch.object(r,'start') as start:
   with self.assertRaisesRegex(ValueError,'runtime_exited'):r.argv('/request','/cwd',{})
  start.assert_not_called();r.close()
  with self.assertRaisesRegex(ValueError,'runtime_retired'):r.argv('/request','/cwd',{})
 def test_single_shot_retirement_closes_runtime_even_when_scm_refuses(self):
  owner=s.Nad1Owner({'operation':'a'*32,'report':'/unused/result.json','application':{'environment':{'root':'/unused'}}},None,None,lambda:False)
  owner.runtime=Mock()
  with patch.object(owner,'_retire_service',return_value=False):self.assertFalse(owner.retire())
  owner.runtime.close.assert_called_once()

class CleanupAndPipeTests(unittest.TestCase):
 def test_runtime_pipes_drain_after_dispatch_and_after_privacy_cutoff(self):
  import os,subprocess,sys,time,threading
  with tempfile.TemporaryDirectory() as tmp:
   root=pathlib.Path(tmp);go=root/'go';done=root/'done'
   owner=types.SimpleNamespace(root=root,directory=root,op='a'*32,diagnostic_privacy=False,spec={'application':{'environment':{'runner':{'entry_point':'/fixture','files':[]}}}})
   runtime=s.Nad1Runtime(owner);runtime.ready=True
   runtime.capture=s.PrivateCapture(root/'capture',1024,600)
   child=subprocess.Popen([sys.executable,'-c',
    'import os,time,pathlib\n'
    'for i in range(128):\n os.write(1,b"x"*8192);os.write(2,b"y"*8192)\n'
    f'pathlib.Path({str(done)!r}).touch()\n'
    f'while not pathlib.Path({str(go)!r}).exists():time.sleep(.01)\n'
    'for i in range(128):\n os.write(1,b"PRIVATE_LOGIN_DATA"*512);os.write(2,b"PRIVATE_LOGIN_DATA"*512)\n'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
   runtime.child=child
   for name in ('stdout','stderr'):
    pipe=getattr(child,name);os.set_blocking(pipe.fileno(),False);runtime.sel.register(pipe,s.selectors.EVENT_READ,name)
   runtime.pump_thread=threading.Thread(target=runtime._pump,daemon=True);runtime.pump_thread.start()
   try:
    limit=time.monotonic()+5
    # No command dispatch or manual drain is needed for the writer to progress.
    while not done.exists() and time.monotonic()<limit:time.sleep(.01)
    self.assertTrue(done.exists(),'runtime output blocked after dispatch')
    runtime.suppress_diagnostics();go.touch();self.assertEqual(child.wait(timeout=5),0)
    limit=time.monotonic()+2
    while runtime.discarded_private<2*128*512*len(b'PRIVATE_LOGIN_DATA') and time.monotonic()<limit:time.sleep(.01)
    self.assertGreater(runtime.discarded_private,0)
    self.assertEqual(runtime.output,b'');self.assertIsNone(runtime.pump_error)
   finally:
    if child.poll() is None:child.kill();child.wait()
    runtime.close()
   self.assertNotIn(b'PRIVATE_LOGIN_DATA',(root/'capture').read_bytes())
   self.assertLessEqual((root/'capture').stat().st_size,1024)
 def test_all_runtime_close_steps_attempted_on_enotempty_and_pipe_error(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=pathlib.Path(tmp);owner=types.SimpleNamespace(spec={'application':{'environment':{'runner':{'entry_point':'/fixture'}}}})
   r=s.Nad1Runtime(owner);r.directory=root;r.socket=root/'socket';r.socket.touch();(root/'unexpected').touch()
   r.child=Mock();r.child.stdin.close.side_effect=OSError('generated pipe close');r.capture=Mock()
   r.close();r.close()
   r.child.stderr.close.assert_called_once();r.capture.close.assert_called_once()
   self.assertEqual(r.close_errors,['stdin_OSError','directory_OSError'])
   self.assertFalse(r.socket.exists());self.assertTrue((root/'unexpected').exists())
 def test_resource_failure_never_skips_either_owner_cleanup_or_terminal_result(self):
  import json,hashlib,signal
  for application,original_failure in ((False,True),(True,True),(False,False)):
   with self.subTest(application=application,original_failure=original_failure),tempfile.TemporaryDirectory() as tmp:
    root=pathlib.Path(tmp).resolve();(root/'compatdata/pfx').mkdir(parents=True);(root/'compatdata/pfx/system.reg').touch()
    image=root/'fixture.exe';image.write_bytes(b'MZfixture')
    spec={'operation':'a'*32,'application_identity':'b'*64,'software_sha256':'c'*64,'renderer_policy':'software_rendering','report':str(root/'result.json'),'application':{'id':'nad1-source-owned','environment':{'root':str(root),'runner':{'entry_point':'/fixture'}},'files':{'Native Access.exe':{'artifact':{'path':str(image),'sha256':hashlib.sha256(image.read_bytes()).hexdigest()},'size':image.stat().st_size}}}}
    scope=Mock();scope.members.return_value=[];ledger=Mock();ledger.cleanup.return_value=True
    library=Mock();library.prctl.return_value=0
    def ensure(owner,*_):
     owner.runtime=s.Nad1Runtime(owner);r=owner.runtime;r.directory=root/'runtime';r.directory.mkdir();r.socket=r.directory/'socket';r.socket.touch();(r.directory/'unexpected').touch()
     if original_failure:raise ValueError('original_readiness_failure')
    signals=[signal.getsignal(x) for x in (signal.SIGTERM,signal.SIGINT)]
    try:
     with patch.object(s,'CompanionCgroup',return_value=scope),patch.object(s,'InstallerLedger',return_value=ledger),patch.object(s.ctypes,'CDLL',return_value=library),patch.object(s.Nad1Owner,'ensure',ensure),patch.object(s.Nad1Owner,'_retire_service',return_value=True):
      if application:s.renderer_run(spec,dependency={'daemon':{}},dependency_fixture={'installer':'Setup.exe','daemon':'NTKDaemon.exe','installer_sha256':'d'*64,'installer_size':256})
      else:s.nad1_owned(spec,fixture={'installer':'Setup.exe','daemon':'NTKDaemon.exe','installer_sha256':'d'*64,'installer_size':256})
     ledger.cleanup.assert_called_once()
     v=json.loads((root/'result.json').read_bytes())
     self.assertEqual(v['error'],'original_readiness_failure' if original_failure else 'dependency_runtime_close_failed');self.assertTrue(v['cleanup_confirmed']);self.assertEqual(v['owned_live'],0)
     self.assertIn('directory_OSError',v['dependency']['runtime_close_errors'])
    finally:
     for sig,old in zip((signal.SIGTERM,signal.SIGINT),signals):signal.signal(sig,old)

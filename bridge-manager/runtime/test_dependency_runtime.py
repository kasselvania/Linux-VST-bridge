"""Operation-owned Proton-session authority, with no Wine or proprietary application."""
import hashlib,pathlib,sys,tempfile,types,unittest
from unittest.mock import Mock,patch
import session as s

class RuntimeTests(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
  root=pathlib.Path(self.tmp.name).resolve()
  (root/'home').mkdir();(root/'compatdata/pfx').mkdir(parents=True)
  self.owner=types.SimpleNamespace(root=root,directory=root,op='a'*32,token='b'*64,retiring=False,diagnostic_privacy=False,
   cancelled=lambda:False,ledger=Mock(),spec={'installer_launch':{'path':'/fixed/adapter.exe'},
   'application':{'environment':{'root':str(root),'runner':{'entry_point':'/fixed/runtime/_v2-entry-point',
   'proton':'/fixed/proton','files':[]}}}})
  self.runtime=s.Nad1Runtime(self.owner);self.addCleanup(self.runtime.close)
 def artifacts(self):
  return [{'path':str(path),'sha256':str(index)*64} for index,path in enumerate((
   pathlib.Path(self.runtime.runner['entry_point']),pathlib.Path(self.runtime.runner['proton']),
   self.runtime.client,self.runtime.interface,self.runtime.wine),1)]
 def test_missing_or_duplicate_exact_tool_identity_refuses(self):
  artifacts=self.artifacts()
  for index,name in enumerate(('entry','proton','client','interface','wine')):
   for files in (artifacts[:index]+artifacts[index+1:],artifacts+[artifacts[index]]):
    self.runtime.runner['files']=files
    with self.subTest(name=name,count=len(files)),self.assertRaisesRegex(ValueError,name+'_identity'),patch.object(s,'verify') as verify:
     self.runtime.verify_tools()
    self.assertEqual(verify.call_count,index)
 def test_exact_entry_proton_client_interface_and_wine_are_verified(self):
  artifacts=self.artifacts();self.runtime.runner['files']=artifacts
  with patch.object(s,'verify') as verify:self.runtime.verify_tools()
  self.assertEqual([call.args[0] for call in verify.call_args_list],artifacts)
  self.assertEqual(self.runtime.tool_sha256,{name:artifact['sha256'] for name,artifact in zip(('entry','proton','client','interface','wine'),artifacts)})
  with patch.object(s,'verify',side_effect=RuntimeError('changed')):
   with self.assertRaises(RuntimeError):self.runtime.verify_tools()
 def test_root_is_exact_interface_owned_proton_anchor(self):
  r=self.runtime;request=r._runtime_request()
  self.assertEqual(request.read_bytes().decode('utf-16le').splitlines(),['NAD1_RUNTIME_V1','a'*32,'b'*64])
  self.assertEqual(r._root_argv(request),['/fixed/runtime/_v2-entry-point','--verb=run','--',str(r.interface),'proton',
   '/fixed/proton','runinprefix','/fixed/adapter.exe',s.windows(request,self.owner.root/'compatdata/pfx')])
  env=r._root_environment({'PATH':'/usr/bin:/bin','CLOSED':'yes'})
  self.assertEqual(env['STEAM_COMPAT_LAUNCHER_SERVICE'],'proton')
  self.assertEqual(env['PATH'],str(r.interface.parent)+':/usr/bin:/bin');self.assertEqual(env['CLOSED'],'yes')
 def test_startup_requires_one_exact_anchor_and_one_private_bus_name(self):
  op='a'*32;token='b'*64;frame=f'NAD1_RUNTIME_V1 {op} {token}\n'.encode()
  hint=b'Starting program\n\n\t--bus-name=:1.10823 \\\n\t-- \\\n\tbash\n'
  self.assertEqual(s.Nad1Runtime._startup_binding(frame,hint,op,token),':1.10823')
  self.assertIsNone(s.Nad1Runtime._startup_binding(frame,b'unrelated',op,token))
  self.assertIsNone(s.Nad1Runtime._startup_binding(b'',hint,op,token))
  with self.assertRaisesRegex(ValueError,'anchor_identity'):s.Nad1Runtime._startup_binding(frame.replace(op.encode(),b'c'*32),hint,op,token)
  for stdout,stderr in ((frame+frame,hint),(frame,hint+hint)):
   with self.assertRaisesRegex(ValueError,'startup_ambiguity'):s.Nad1Runtime._startup_binding(stdout,stderr,op,token)
 def test_production_start_and_close_bind_one_long_lived_anchor(self):
  root=self.owner.root;runtime_root=root/'runtime';runner_root=root/'runner'
  (runtime_root/'pressure-vessel/bin').mkdir(parents=True);(runner_root/'files/bin').mkdir(parents=True)
  entry=runtime_root/'_v2-entry-point';proton=runner_root/'proton';wine=runner_root/'files/bin/wine'
  client=runtime_root/'pressure-vessel/bin/steam-runtime-launch-client';interface=runtime_root/'pressure-vessel/bin/steam-runtime-launcher-interface-0'
  source=(f'#!{sys.executable}\nimport sys\n'
   'sys.stderr.write("\\t--bus-name=:1.77 \\\\\\n");sys.stderr.flush()\n'
   f'sys.stdout.write("NAD1_RUNTIME_V1 {"a"*32} {"b"*64}\\n");sys.stdout.flush()\n'
   'sys.stdin.buffer.read()\n')
  entry.write_text(source)
  for path in (entry,proton,wine,client,interface):
   if not path.exists():path.write_bytes(b'source-owned')
   path.chmod(0o700)
  runner={'entry_point':str(entry),'proton':str(proton),'files':[{'path':str(path),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()} for path in (entry,proton,client,interface,wine)]}
  self.owner.spec['application']['environment']['runner']=runner
  runtime=s.Nad1Runtime(self.owner)
  try:
   runtime.start({'PATH':'/usr/bin:/bin'})
   self.assertTrue(runtime.ready);self.assertEqual(runtime.bus_name,':1.77');self.assertIsNone(runtime.child.poll())
   self.assertEqual(self.owner.ledger.launcher.call_args.args[1],'dependency_runtime')
   runtime.close();self.assertEqual(runtime.child.returncode,0);self.assertEqual(runtime.close_errors,[])
  finally:runtime.close()
 def test_commands_reuse_exact_proton_session_and_only_forward_closed_diagnostics(self):
  r=self.runtime;r.bus_name=':1.42';r.ready=True;r.child=Mock(returncode=None)
  with patch.object(r,'start') as start,patch.object(r,'drain'),patch.object(r,'verify_tools'):
   a=r.argv(self.owner.root/'one.private',self.owner.root,{'EVIL':'value'})
   b=r.argv(self.owner.root/'two.private',self.owner.root/'home',{})
  start.assert_not_called();self.assertEqual(a[0],str(r.client));self.assertEqual(a[1],b[1])
  self.assertEqual(a[1],'--bus-name=:1.42')
  self.assertNotIn('EVIL',' '.join(a));self.assertNotIn('--clear-env',a)
  self.assertEqual([x for x in a if x.startswith('--pass-env=')],['--pass-env='+x for x in r.DEBUG_KEYS])
  self.assertEqual(a[-3:-1],[str(r.wine),'/fixed/adapter.exe'])
  self.assertIn('--directory='+str(self.owner.root/'home'),b)
  # Only the retained root invokes Proton. Inserted commands invoke the
  # verified Wine image directly inside its exact command service.
  self.assertNotIn('runinprefix',a)
  self.assertNotIn(':1.42',str(r.value()))
 def test_dead_or_retired_container_never_falls_back_to_another_container(self):
  r=self.runtime;r.child=Mock(returncode=1);r.ready=True;r.bus_name=':1.42'
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
   owner=types.SimpleNamespace(root=root,directory=root,op='a'*32,diagnostic_privacy=False,spec={'application':{'environment':{'runner':{'entry_point':'/fixture','proton':'/proton','files':[]}}}})
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
    self.assertEqual(runtime.startup_output,{'stdout':bytearray(),'stderr':bytearray()});self.assertIsNone(runtime.pump_error)
   finally:
    if child.poll() is None:child.kill();child.wait()
    runtime.close()
   self.assertNotIn(b'PRIVATE_LOGIN_DATA',(root/'capture').read_bytes())
   self.assertLessEqual((root/'capture').stat().st_size,1024)
 def test_all_runtime_close_steps_attempted_on_enotempty_and_pipe_error(self):
  owner=types.SimpleNamespace(spec={'application':{'environment':{'runner':{'entry_point':'/fixture','proton':'/proton'}}}})
  r=s.Nad1Runtime(owner);r.child=Mock(returncode=0);r.child.poll.return_value=0
  r.child.stdin.close.side_effect=OSError('generated pipe close');r.capture=Mock();r.capture.close.side_effect=OSError('generated capture close')
  r.close();r.close()
  r.child.stderr.close.assert_called_once();r.capture.close.assert_called_once()
  self.assertEqual(r.close_errors,['stdin_OSError','capture_OSError'])
 def test_resource_failure_never_skips_either_owner_cleanup_or_terminal_result(self):
  import json,hashlib,signal
  for application,original_failure in ((False,True),(True,True),(False,False)):
   with self.subTest(application=application,original_failure=original_failure),tempfile.TemporaryDirectory() as tmp:
    root=pathlib.Path(tmp).resolve();(root/'compatdata/pfx').mkdir(parents=True);(root/'compatdata/pfx/system.reg').touch()
    image=root/'fixture.exe';image.write_bytes(b'MZfixture')
    spec={'operation':'a'*32,'application_identity':'b'*64,'software_sha256':'c'*64,'renderer_policy':'software_rendering','report':str(root/'result.json'),'application':{'id':'nad1-source-owned','environment':{'root':str(root),'runner':{'entry_point':'/fixture','proton':'/proton','files':[]}},'files':{'Native Access.exe':{'artifact':{'path':str(image),'sha256':hashlib.sha256(image.read_bytes()).hexdigest()},'size':image.stat().st_size}}}}
    authority={'origin':{'kind':'qualified_recovered_installation'},'service':s.NAD1_SERVICE,'listeners':[5146,5563],
     'installer':{'sha256':'d'*64,'size':256},'daemon':{}}
    if application:spec['dependency_session']=authority
    scope=Mock();scope.members.return_value=[];ledger=Mock();ledger.cleanup.return_value=True
    library=Mock();library.prctl.return_value=0
    def ensure(owner,*_):
     owner.runtime=s.Nad1Runtime(owner);r=owner.runtime;r.ready=True;r.capture=Mock();r.capture.close.side_effect=OSError('generated capture close')
     if original_failure:raise ValueError('original_readiness_failure')
    signals=[signal.getsignal(x) for x in (signal.SIGTERM,signal.SIGINT)]
    try:
     with patch.object(s,'CompanionCgroup',return_value=scope),patch.object(s,'InstallerLedger',return_value=ledger),patch.object(s.ctypes,'CDLL',return_value=library),patch.object(s,'nad1_session_admitted_inputs',return_value=authority),patch.object(s.Nad1Owner,'ensure',ensure),patch.object(s.Nad1Owner,'_retire_service',return_value=True):
      if application:s.renderer_run(spec,dependency=authority,dependency_fixture={'installer':'Setup.exe','daemon':'NTKDaemon.exe','installer_sha256':'d'*64,'installer_size':256,'session_directory':str(root),'session_qualification':{}})
      else:s.nad1_owned(spec,fixture={'installer':'Setup.exe','daemon':'NTKDaemon.exe','installer_sha256':'d'*64,'installer_size':256})
     ledger.cleanup.assert_called_once()
     v=json.loads((root/'result.json').read_bytes())
     self.assertEqual(v['error'],'original_readiness_failure' if original_failure else 'dependency_runtime_close_failed');self.assertTrue(v['cleanup_confirmed']);self.assertEqual(v['owned_live'],0)
     self.assertIn('capture_OSError',v['dependency']['runtime_close_errors'])
    finally:
     for sig,old in zip((signal.SIGTERM,signal.SIGINT),signals):signal.signal(sig,old)

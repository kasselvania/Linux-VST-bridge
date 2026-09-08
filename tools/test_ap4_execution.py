"""AP4 uses the actual classified worker; substitute only platform/process work."""
import pathlib,sys
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parent))
import json,sys,types,unittest,copy
from unittest.mock import patch
from test_ap2_execution import AP2ExecutionTests,TOOLS
from test_pc0_proof_adapter import SOURCE_A
from test_proof_cli import cli
import ap4_adapter as a
from classified_proof_backend import ClassifiedProofBackend,DiagnosticPlanAdapter
from proof_execution_policy import ExecutionClass,LiveRequest,load_authority,authorize_live_request

class AP4ExecutionTests(AP2ExecutionTests):
 def setUp(self):
  super().setUp();old=self.runtime
  plan=a.descriptor(ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE,old.artifact,old.native)
  r=a.AP4Runtime(old.ports,plan,old.artifact,old.native,repository=TOOLS.parent,proof_root=self.mac_proof)
  self.runtime=r;self.plan=plan
  self.stack.enter_context(patch.object(r,'_dependencies',old._dependencies));self.stack.enter_context(patch.object(r,'_local_preflight'))
  for name in ('ap3_contract','ap3_worker_support','ap4_contract','ap4_worker_support'):
   module=types.ModuleType(name);module.__file__='<'+name+'>';module._pc0_source_sha256=a.sha256_bytes(a.SUPPORT[name].encode())
   self.stack.enter_context(patch.dict(sys.modules,{name:module}));exec(compile(a.SUPPORT[name],module.__file__,'exec'),module.__dict__)
  self.profile=sys.modules['ap4_worker_support'];self.stack.enter_context(patch.object(self.profile,'verify_host',return_value=self.host));self.stack.enter_context(patch.object(self.profile,'bind_client'))
  def scan(*args,**kwargs):
   return dict(records=[],raw_exit=1,classification='scanner_failed',cleanup={'owned_descendants_zero':True,'process_group_empty':True},caller={'records':[dict(event='ap4_host_compared',frames_per_callback=128,active_frames=1440000,samples=2882048,max_error=0.,callback_max_ns=9000,callback_overruns=0,input_fnv1a64=123,output_fnv1a64=456,envelope_hex='a'*232,payload_hex='b'*24,controller_gain=.25),dict(event='ap4_proxy_stats',fault=1,first_position=1024,processed=4,request_high=5,result_high=0,position=1024,epoch=2),dict(event='ap4_host_error',stage='process',detail='original injected response fault: control disconnected/IO',last_position=1024),dict(event='ap4_native_error',operation='get',stage='failed_instance',fault=1,first_position=1024,processed=4,callback_rejections=1,detail='first native delayed output underflow')],'raw_exit':1,'cleanup':{'owned_descendants_zero':True,'process_group_empty':True}})
  self.profile.configure(False)
  self.batch_supervise=self.profile.supervise
  self.scan=self.stack.enter_context(patch.object(self.profile,'supervise',side_effect=scan))
  self.adapter=DiagnosticPlanAdapter(descriptor=plan,preflight=r.preflight,reconcile=r.reconcile,invoke=r.invoke_diagnostic,admit=r.admit)
  self.backend=ClassifiedProofBackend(self.base/'state',{plan.plan_id:self.adapter})
  fields=dict(self.auth.fields);fields.pop('acceptance_candidate_identity',None)
  fields.update(status='active_diagnostic_campaign',authority_phase='proof_harness_maintenance',change_class='PROOF_HARNESS_MAINTENANCE',product_implementation_authorized=False,maintenance_implementation_authorized=True,permitted_execution_class='DIAGNOSTIC_NON_AUTHORITATIVE',acceptance_eligible=False,authorized_plan_id=plan.plan_id,authorized_product_contract_identity=plan.product_contract_identity,authorized_product_contract_sha256=plan.product_contract_sha256,authorized_plan_content_sha256=plan.plan_content_sha256,diagnostic_campaign_identity='a'*64,diagnostic_batch_budget=10)
  self.authority.write_text('## Authority\n```yaml\n'+'\n'.join(f'{k}: {str(v).lower() if isinstance(v,bool) else v}' for k,v in fields.items())+'\n```\n');self.auth=load_authority(self.authority)
  self.delegation=authorize_live_request(self.auth,LiveRequest(ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE,SOURCE_A,plan.plan_id,'a'*64));self.current_reservation=self.backend._reservation_identity(self.delegation)
  self.args=['diagnose','--authority',str(self.authority),'--source',SOURCE_A,'--plan',plan.plan_id,'--campaign','a'*64]
  self.stack.enter_context(patch.object(cli,'_production_adapters',return_value=({plan.plan_id:self.adapter},types.SimpleNamespace(last_failure=None))))
 def transaction(self):
  paths=list((self.base/'state').rglob('transaction.json'));self.assertEqual(len(paths),1);return json.loads(paths[0].read_bytes())
 def test_ap4_failed_proxy_retained_before_retirement(self):
  self.execute_cli();doc=self.transaction()['observation']['payload']['document']
  self.assertEqual(doc['schema'],'linux-vst-bridge-ap4-reservation/v1')
  self.assertEqual(doc['cleanup'],'COMPLETE');self.assertEqual(self.retire.call_count,1)
  self.assertIn('original injected response fault',json.dumps(doc))
  records=doc['summary']['troubleshooting']['observation']['caller']['records']
  self.assertEqual(records[0]['frames_per_callback'],128)
  self.assertEqual(records[0]['output_fnv1a64'],456)
  self.assertEqual(records[0]['envelope_hex'],'a'*232)
  self.assertEqual(records[0]['payload_hex'],'b'*24)
  self.assertEqual(records[1]['fault'],1);self.assertEqual(records[1]['first_position'],1024)
  self.assertEqual(records[1]['epoch'],2);self.assertEqual(records[2]['last_position'],1024)
  self.assertIn('control disconnected',records[2]['detail'])
  self.assertEqual(records[3]['event'],'ap4_native_error')
  self.assertEqual(records[3]['fault'],1)
  self.assertEqual(records[3]['first_position'],1024)
  self.assertEqual(records[3]['callback_rejections'],1)
  self.assertEqual(records[3]['detail'],'first native delayed output underflow')
  self.assertFalse(doc['binding']['acceptance_eligible']);self.assertFalse(self.rendered)
 def test_ap4_lost_ack_reconciles_without_replay(self):
  self.local_worker.lose_ack=True;self.execute_cli();self.execute_cli();self.assertEqual(self.scan.call_count,1)
 def test_ap4_read_only_preflight_does_not_run(self):
  result=self.execute_cli(['--preflight-only']);self.assertEqual(result['state'],'ready');self.assertEqual(self.scan.call_count,0)

 def test_ap4_gui_setup_crash_retains_streams_and_missing_report_before_cleanup(self):
  self.caller_retention_case('setup_crash')
 def test_ap4_gui_allocator_abort_survives_missing_report_and_cleanup(self):
  self.caller_retention_case('allocator_abort')
 def test_ap4_gui_original_observation_survives_report_and_cleanup_failure(self):
  self.caller_retention_case('cleanup_failure')
 def test_ap4_gui_observation_is_checkpointed_before_cleanup_removes_report(self):
  self.caller_retention_case('observed')
 def test_ap4_gui_setup_crash_lost_ack_does_not_repeat_execution(self):
  self.local_worker.lose_ack=True
  self.caller_retention_case('setup_crash')
  self.execute_cli();self.assertEqual(self.scan.call_count,1)
 def test_ap4_gui_post_cleanup_reporting_failure_keeps_earlier_native_checkpoint(self):
  self.caller_retention_case('late_report_failure')
 def test_ap4_gui_diagnostic_persistence_failure_still_cleans_up(self):
  self.caller_retention_case('checkpoint_failure')
 def test_ap4_caller_stream_excerpt_is_bounded_and_rejects_symlinks(self):
  import hashlib
  companion=self.profile.companion;path=self.base/'bounded-stream'
  raw=b'error: old overwritten tail\n'+b'x'*32768+b'\nfatal error: '+b'e'*2000+b'\n'
  path.write_bytes(raw);digest,detail=companion.retained_stream(path)
  self.assertEqual(digest,hashlib.sha256(raw).hexdigest())
  self.assertLessEqual(len(detail),768);self.assertNotIn('old overwritten tail',detail)
  link=self.base/'stream-symlink';link.symlink_to(path)
  with self.assertRaisesRegex(RuntimeError,'unsafe caller diagnostic stream'):companion.retained_stream(link)
  with path.open('wb') as stream:stream.truncate(8*1024*1024+1)
  with self.assertRaisesRegex(RuntimeError,'exceeds read bound'):companion.retained_stream(path)
 def caller_retention_case(self,case):
  companion=self.profile.companion;clean={'owned_descendants_zero':True,'process_group_empty':True}
  session=self.base/'retention-session';session.mkdir()
  self.create.side_effect=lambda *args,**kwargs:types.SimpleNamespace(session=session)
  histories=[];windows=[];reports=[]
  original_record=dict(event='ap4_sample_comparison',samples=512,max_error=0.)
  def actual_caller(environment,*,mode,checkpoint,profile):
   report=environment.session/'ap3-gui-report.jsonl';crash=environment.session/'BITWIG_ENGINE_CRASH.txt'
   setup_failed=case in {'setup_crash','checkpoint_failure','allocator_abort'}
   process=types.SimpleNamespace(pid=123,poll=lambda:134 if setup_failed else 0)
   def spawn(*args,**kwargs):
    kwargs['stdout'].write(b'ignored normal startup\nAssertion failed: engine setup /home/private/file 0xdeadbeef pid=912\n')
    kwargs['stderr'].write(b'malloc(): unsorted double linked list corrupted\n' if case=='allocator_abort' else b'fatal error: endpoint not ready token=private-value user@example.com 192.0.2.1\n')
    crash.write_text('Assertion failed: native engine setup\n')
    if not setup_failed:
     (environment.session/'ap1.control').write_bytes(b'local substitute')
     report.write_text(json.dumps(original_record)+'\n'+('invalid trailing report\n' if case=='cleanup_failure' else ''))
    return process
   def read_report():
    reports.append(True)
    if not report.is_file():raise RuntimeError('bounded native Bitwig report missing')
    return report.read_bytes()
   def cleanup(*args):
    self.assertEqual(args[1],[(123,10)])
    self.assertEqual(histories[-1][0],'native_caller_before_cleanup')
    self.assertFalse(histories[-1][1]['cleanup']['process_group_empty'])
    report.unlink(missing_ok=True);crash.unlink()
    if case=='cleanup_failure':raise RuntimeError('separate owned-process cleanup failure')
    return clean
   def win(capture):
    windows.append(True)
    if case=='cleanup_failure':raise ValueError('original Windows supervisor failure')
    return dict(records=[],classification='scanner_completed',raw_exit=0,cleanup=clean)
   def capture(*args):
    histories.append(copy.deepcopy(args[:2]));checkpoint(*args)
    if case=='checkpoint_failure' and args[0]=='native_caller_before_cleanup':raise OSError('separate checkpoint persistence failure')
   def late_report():raise ValueError('late GUI note failure')
   with patch.object(companion.subprocess,'Popen',side_effect=spawn),patch.object(companion.inherited,'process_identity',return_value={'pid':123,'start_ticks':10}),patch.object(companion.inherited,'cleanup_process',side_effect=cleanup):
    return self.actual_profile_supervise(environment,mode=mode,checkpoint=capture,profile=profile,
     caller_command=['substituted'],caller_env={'LOCAL':'true'},caller_report=late_report if case=='late_report_failure' else read_report,
     caller_checkpoint_report=read_report if case=='late_report_failure' else None,windows_run=win,accepted_events={'ap4_sample_comparison'})
  self.scan.side_effect=actual_caller;self.execute_cli()
  doc=self.transaction()['observation']['payload']['document'];trouble=doc['summary']['troubleshooting']
  self.assertIsNotNone(trouble['observation'],trouble)
  caller=trouble['observation']['caller']
  setup_failed=case in {'setup_crash','checkpoint_failure','allocator_abort'}
  self.assertEqual(caller['raw_exit'],134 if setup_failed else 0)
  self.assertEqual(caller['records'],[] if setup_failed else [original_record])
  self.assertEqual(len(reports),1);self.assertEqual(len(windows),0 if setup_failed else 1)
  self.assertIn('Assertion failed: engine setup',caller['stdout_detail'])
  self.assertIn('unsorted double linked list corrupted' if case=='allocator_abort' else 'endpoint not ready',caller['stderr_detail'])
  self.assertIn('native engine setup',caller['crash_detail'])
  for key in ('stdout_sha256','stderr_sha256','crash_sha256'):self.assertEqual(len(caller[key]),64)
  if setup_failed:
   self.assertIn('native caller exited during setup',caller['error'][0]['detail'])
   self.assertIn('bounded native Bitwig report missing',caller['reporting_error'][0]['detail'])
   self.assertEqual(caller['error'][0]['type'],'RuntimeError')
   self.assertEqual(caller['error'][0]['frames'][-1]['module'],'tools/ap1_worker_support.py')
   self.assertEqual(caller['error'][0]['frames'][-1]['function'],'supervise')
   if case=='checkpoint_failure':self.assertIn('separate checkpoint persistence failure',caller['retention_error'][0]['detail'])
  elif case=='cleanup_failure':
   self.assertIn('original Windows supervisor failure',caller['error'][0]['detail'])
   self.assertEqual(caller['reporting_error'][0]['type'],'JSONDecodeError')
   self.assertIn('separate owned-process cleanup failure',caller['cleanup_error'][0]['detail'])
  elif case=='late_report_failure':self.assertIn('late GUI note failure',caller['reporting_error'][0]['detail'])
  else:self.assertNotIn('reporting_error',caller)
  self.assertEqual(caller['cleanup']['process_group_empty'],case!='cleanup_failure')
  self.assertEqual(self.retire.call_count,0 if case=='cleanup_failure' else 1)
  text=json.dumps(trouble)
  for private in ('/home/private','0xdeadbeef','pid=912','private-value','user@example.com','192.0.2.1'):self.assertNotIn(private,text)
  self.assertFalse(doc['binding']['acceptance_eligible']);self.assertFalse(self.rendered)


 def test_ap4_gui_note_arrives_after_clean_caller_exit(self):
  self.gui_note_case('delayed')
 def test_ap4_gui_note_timeout_preserves_native_records(self):
  self.gui_note_case('missing')
 def test_ap4_failed_gui_does_not_wait_for_observation(self):
  self.gui_note_case('failed')
 def gui_note_case(self,case):
  profile=self.profile.previous;session=self.base/('gui-'+case);session.mkdir()
  env=types.SimpleNamespace(session=session,run_id='f'*32)
  root=self.base/'gui-fixture';root.mkdir();project=root/'test.bwproject';project.write_bytes(b'private-test-fixture')
  native=self.base/'native-gui';relative='AGainQueuedBridge.vst3/Contents/x86_64-linux/AGainQueuedBridge.so'
  for path in (root/'plugins'/relative,native/relative):path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(b'exact-native')
  records=[dict(event='ap4_native_state',operation='get',gain=.25),dict(event='ap3_proxy_lifecycle',clean=case!='failed',blocks=4)]
  report=session/'ap3-gui-report.jsonl';report.write_text(''.join(json.dumps(r)+'\n' for r in records))
  note=dict(event='ap4_bitwig_ui',run_id=env.run_id,case='bitwig_first',playback=True,saved=True,quit=True,responsive=True,moonlight_control=True)
  ticks=[0.];sleeps=[]
  def sleep(seconds):
   sleeps.append(seconds);ticks[0]+=30.
   if case=='delayed':(session/'bitwig_first-ui.json').write_text(json.dumps(note)+'\n')
  def completed_caller(*args,**kwargs):
   # Native capture happens before cleanup without waiting for a GUI note.
   self.assertEqual(kwargs['post_gate_seconds'],180)
   self.assertEqual(kwargs['caller_env'],{'HOME':str(self.base),'PATH':'/usr/bin:/bin','DISPLAY':':fixture'})
   self.assertIn('--cwd='+str(self.base),kwargs['caller_command'])
   self.assertIn('--env=LVB_AP2_SESSION_DIR='+str(session),kwargs['caller_command'])
   self.assertIn('--filesystem='+str(session),kwargs['caller_command'])
   raw=kwargs['caller_checkpoint_report']()
   self.assertEqual([json.loads(line) for line in raw.splitlines()],records)
   self.assertEqual(sleeps,[])
   report.unlink() # Cached facts survive removal before the post-cleanup note.
   raw=kwargs['caller_report']();return [json.loads(line) for line in raw.splitlines()]
  def command(args,**kwargs):
   return '8a048e733e74dda8b897339436153a2d5df952f29d362dfcaca9d8e0d6f6c231\n' if args[0]=='flatpak' else 'DISPLAY=:fixture\nUNRELATED_SECRET=must-not-propagate\n'
  with patch.object(profile.subprocess,'check_output',side_effect=command),patch.object(profile.inherited,'controlled_environment',return_value={'HOME':str(self.base),'PATH':'/usr/bin:/bin','STEAM_COMPAT_DATA_PATH':'private-prefix','SteamAppId':'0','PRESSURE_VESSEL_VARIABLE_DIR':'private-runtime','TMPDIR':'stage-temp'}),patch.object(profile,'progress'),patch.object(profile,'real_home',return_value=self.base),patch.object(profile.companion,'supervise',side_effect=completed_caller),patch.object(profile.time,'monotonic',side_effect=lambda:ticks[0]),patch.object(profile.time,'sleep',side_effect=sleep):
   result=profile.gui(env,mode='local',checkpoint=lambda *v:None,profile=self.profile,label='bitwig_first',root=root,project=project,native=native,ui_note_wait_seconds=60,post_gate_seconds=180)
  self.assertEqual(result[:2],records)
  self.assertEqual(result[2:],[note] if case=='delayed' else [])
  self.assertEqual(len(sleeps),{'delayed':1,'missing':2,'failed':0}[case])
  self.assertFalse(report.exists());self.assertTrue(project.is_file())

 def test_ap4_gui_deadline_allows_bounded_interaction_then_cleans_up(self):
  self.deadline_case(180,3)
 def test_ap4_default_stage_deadline_remains_120_seconds(self):
  self.deadline_case(None,2)
 def test_ap4_gui_window_does_not_extend_plugin_call_deadline(self):
  self.deadline_case(180,2,in_flight=True)
 def test_ap4_invalid_stage_deadline_rejected_before_spawn(self):
  from unittest.mock import MagicMock
  for value in (0,-1,181,float('inf'),float('nan'),True,'180'):
   with self.subTest(value=value),patch.object(self.diagnostic.subprocess,'Popen') as spawn:
    with self.assertRaisesRegex(RuntimeError,'post-gate stage deadline'):
     self.actual_supervise(MagicMock(),post_gate_seconds=value)
    spawn.assert_not_called()
 def deadline_case(self,limit,pumps,in_flight=False):
  from unittest.mock import MagicMock
  import contextlib
  session=self.base/'deadline';session.mkdir()
  env=types.SimpleNamespace(session=session,run_id='a'*32,marker={'fixture':'again'})
  root=MagicMock(pid=123,returncode=None);root.poll.return_value=None
  selector=MagicMock();ticks=[0.];calls=[];captures=[]
  expected=b'exact-ready-binding'
  def pump(*args):
   streams=args[1];calls.append(ticks[0])
   if len(calls)==1:
    (session/('b'*32+'.ready')).write_bytes(expected)
    streams.records.append({'event':'lifecycle','state':'readiness_announced'})
    if in_flight:
     streams.in_flight_at=.1
     streams.in_flight={'operation':'load_library'}
   else:ticks[0]=121. if len(calls)==2 else 181.
  def cleanup(*args):
   root.returncode=-15
   return {'owned_descendants_zero':True,'process_group_empty':True}
  with contextlib.ExitStack() as stack:
   for name,value in {'verify_diagnostic_runner':self.runtime_observation,'verify_environment':None,
    'handshake':expected,'command_vector':['inert-test-command'],'controlled_environment':{},
    'process_identity':{'pid':123,'start_ticks':1},'descendants':[],'descendant_identities':[],'topology':{},
    'protected_snapshot':self.protected}.items():
    stack.enter_context(patch.object(self.diagnostic,name,return_value=value))
   clean=stack.enter_context(patch.object(self.diagnostic,'cleanup_process',side_effect=cleanup))
   stack.enter_context(patch.object(self.diagnostic.subprocess,'Popen',return_value=root))
   stack.enter_context(patch.object(self.diagnostic.selectors,'DefaultSelector',return_value=selector))
   stack.enter_context(patch.object(self.diagnostic.secrets,'token_hex',return_value='b'*32))
   stack.enter_context(patch.object(self.diagnostic.time,'monotonic',side_effect=lambda:ticks[0]))
   stack.enter_context(patch.object(self.diagnostic,'pump',side_effect=pump))
   kwargs={} if limit is None else {'post_gate_seconds':limit}
   result=self.actual_supervise(env,mode=self.common.PC0_MODE,checkpoint=lambda *v:captures.append(v),**kwargs)
  self.assertEqual(len(calls),pumps)
  self.assertIn(result['classification'],('stage_timeout','call_timeout'))
  clean.assert_called_once_with(root,[(123,1)])
  selector.close.assert_called_once()
  self.assertTrue(captures[-1][1]['cleanup']['owned_descendants_zero'])
  self.assertEqual(result['protected_snapshot'],self.protected)

 def test_ap4_batch_stops_before_next_instance_when_normalization_fails(self):
  env=types.SimpleNamespace(session=self.base/'ap4-batch',run_id='f'*32);env.session.mkdir()
  clean={'owned_descendants_zero':True,'process_group_empty':True};saved=[]
  good=dict(classification='scanner_completed',cleanup=clean,records=[])
  with patch.object(self.profile,'core',return_value=good) as run,patch.object(self.profile,'progress'),patch.object(self.profile,'normalize_session',side_effect=RuntimeError('original state admission error')):
   with self.assertRaisesRegex(RuntimeError,'original state admission'):self.batch_supervise(env,mode=self.profile.MODE,profile=self.profile,checkpoint=lambda *v:saved.append(v))
  self.assertEqual(run.call_count,1);self.assertEqual(saved[0][1]['segments'],{'state_capture':good})
  self.assertFalse(list(env.session.iterdir()))

def load_tests(loader,tests,pattern):return unittest.TestSuite(AP4ExecutionTests(n) for n in dir(AP4ExecutionTests) if n.startswith('test_ap4_'))

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
   # Effectful caller has exited; exercise the actual GUI report callback.
   raw=kwargs['caller_report']();return [json.loads(line) for line in raw.splitlines()]
  def command(args,**kwargs):
   return '8a048e733e74dda8b897339436153a2d5df952f29d362dfcaca9d8e0d6f6c231\n' if args[0]=='flatpak' else ''
  with patch.object(profile.subprocess,'check_output',side_effect=command),patch.object(profile.inherited,'controlled_environment',return_value={}),patch.object(profile,'progress'),patch.object(profile,'real_home',return_value=self.base),patch.object(profile.companion,'supervise',side_effect=completed_caller),patch.object(profile.time,'monotonic',side_effect=lambda:ticks[0]),patch.object(profile.time,'sleep',side_effect=sleep):
   result=profile.gui(env,mode='local',checkpoint=lambda *v:None,profile=self.profile,label='bitwig_first',root=root,project=project,native=native,ui_note_wait_seconds=60)
  self.assertEqual(result[:2],records)
  self.assertEqual(result[2:],[note] if case=='delayed' else [])
  self.assertEqual(len(sleeps),{'delayed':1,'missing':2,'failed':0}[case])
  self.assertTrue(report.is_file());self.assertTrue(project.is_file())

 def test_ap4_batch_stops_before_next_instance_when_normalization_fails(self):
  env=types.SimpleNamespace(session=self.base/'ap4-batch',run_id='f'*32);env.session.mkdir()
  clean={'owned_descendants_zero':True,'process_group_empty':True};saved=[]
  good=dict(classification='scanner_completed',cleanup=clean,records=[])
  with patch.object(self.profile,'core',return_value=good) as run,patch.object(self.profile,'progress'),patch.object(self.profile,'normalize_session',side_effect=RuntimeError('original state admission error')):
   with self.assertRaisesRegex(RuntimeError,'original state admission'):self.batch_supervise(env,mode=self.profile.MODE,profile=self.profile,checkpoint=lambda *v:saved.append(v))
  self.assertEqual(run.call_count,1);self.assertEqual(saved[0][1]['segments'],{'state_capture':good})
  self.assertFalse(list(env.session.iterdir()))

def load_tests(loader,tests,pattern):return unittest.TestSuite(AP4ExecutionTests(n) for n in dir(AP4ExecutionTests) if n.startswith('test_ap4_'))

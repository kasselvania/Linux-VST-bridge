"""AP3 uses the actual classified worker; substitute only platform/process work."""
import pathlib,sys
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parent))
import json,sys,types,unittest,copy
from unittest.mock import patch
from test_ap2_execution import AP2ExecutionTests,TOOLS
from test_pc0_proof_adapter import SOURCE_A
from test_proof_cli import cli
import ap3_adapter as a
from classified_proof_backend import ClassifiedProofBackend,DiagnosticPlanAdapter
from proof_execution_policy import ExecutionClass,LiveRequest,load_authority,authorize_live_request

class AP3ExecutionTests(AP2ExecutionTests):
 def setUp(self):
  super().setUp();old=self.runtime
  plan=a.descriptor(ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE,old.artifact,old.native)
  r=a.AP3Runtime(old.ports,plan,old.artifact,old.native,repository=TOOLS.parent,proof_root=self.mac_proof)
  self.runtime=r;self.plan=plan
  self.stack.enter_context(patch.object(r,'_dependencies',old._dependencies));self.stack.enter_context(patch.object(r,'_local_preflight'))
  for name in ('ap3_contract','ap3_worker_support'):
   module=types.ModuleType(name);module.__file__='<'+name+'>';module._pc0_source_sha256=a.sha256_bytes(a.SUPPORT[name].encode())
   self.stack.enter_context(patch.dict(sys.modules,{name:module}));exec(compile(a.SUPPORT[name],module.__file__,'exec'),module.__dict__)
  self.profile=sys.modules['ap3_worker_support'];self.stack.enter_context(patch.object(self.profile,'verify_host',return_value=self.host));self.stack.enter_context(patch.object(self.profile,'bind_client'))
  def scan(*args,**kwargs):
   return dict(records=[],raw_exit=1,classification='scanner_failed',cleanup={'owned_descendants_zero':True,'process_group_empty':True},caller={'records':[dict(event='ap3_host_compared',frames_per_callback=128,active_frames=1440000,samples=2882048,max_error=0.,callback_max_ns=9000,callback_overruns=0,input_fnv1a64=123,output_fnv1a64=456),dict(event='ap3_proxy_stats',fault=1,first_position=1024,processed=4,request_high=5,result_high=0,position=1024,epoch=2),dict(event='ap3_host_error',stage='process',detail='original injected response fault: control disconnected/IO',last_position=1024)],'raw_exit':1,'cleanup':{'owned_descendants_zero':True,'process_group_empty':True}})
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
 def test_ap3_failed_proxy_retained_before_retirement(self):
  self.execute_cli();doc=self.transaction()['observation']['payload']['document']
  self.assertEqual(doc['schema'],'linux-vst-bridge-ap3-reservation/v1')
  self.assertEqual(doc['cleanup'],'COMPLETE');self.assertEqual(self.retire.call_count,1)
  self.assertIn('original injected response fault',json.dumps(doc))
  records=doc['summary']['troubleshooting']['observation']['caller']['records']
  self.assertEqual(records[0]['frames_per_callback'],128)
  self.assertEqual(records[0]['output_fnv1a64'],456)
  self.assertEqual(records[1]['fault'],1);self.assertEqual(records[1]['first_position'],1024)
  self.assertEqual(records[1]['epoch'],2);self.assertEqual(records[2]['last_position'],1024)
  self.assertIn('control disconnected',records[2]['detail'])
  self.assertFalse(doc['binding']['acceptance_eligible']);self.assertFalse(self.rendered)
 def test_ap3_lost_ack_reconciles_without_replay(self):
  self.local_worker.lose_ack=True;self.execute_cli();self.execute_cli();self.assertEqual(self.scan.call_count,1)
 def test_ap3_read_only_preflight_does_not_run(self):
  result=self.execute_cli(['--preflight-only']);self.assertEqual(result['state'],'ready');self.assertEqual(self.scan.call_count,0)

 def test_ap3_batch_preserves_completed_segment_and_stops_on_failure(self):
  env=types.SimpleNamespace(session=self.base/'batch',run_id='f'*32);env.session.mkdir()
  clean={'owned_descendants_zero':True,'process_group_empty':True}
  good=dict(classification='scanner_completed',cleanup=clean,records=[{'state':'core_retained'}])
  failed=dict(classification='scanner_failed',cleanup=clean,records=[{'state':'original_stream_failure'}])
  checkpoints=[]
  with patch.object(self.profile,'core',side_effect=[good,failed]) as run,patch.object(self.profile,'gui') as gui,patch.object(self.profile,'progress'),patch.object(self.profile,'await_stream'),patch.object(self.profile,'normalize_session'):
   result=self.batch_supervise(env,mode=self.profile.MODE,profile=self.profile,checkpoint=lambda *v:checkpoints.append(v))
  self.assertEqual(run.call_count,2);gui.assert_not_called()
  self.assertEqual(result['segments'],{'core':good,'stream_active':failed})
  self.assertEqual(result['classification'],'scanner_failed')
  self.assertTrue((env.session/'ap3-core-closed').is_dir());self.assertFalse((env.session/'ap3-stream_active-closed').exists())
 def test_ap3_batch_does_not_retire_uncontained_segment(self):
  env=types.SimpleNamespace(session=self.base/'batch',run_id='f'*32);env.session.mkdir()
  failed=dict(classification='scanner_completed',cleanup={'owned_descendants_zero':False,'process_group_empty':False})
  with patch.object(self.profile,'core',return_value=failed) as run,patch.object(self.profile,'gui') as gui,patch.object(self.profile,'progress'):
   result=self.batch_supervise(env,mode=self.profile.MODE,profile=self.profile,checkpoint=lambda *v:None)
  self.assertEqual(run.call_count,1);gui.assert_not_called();self.assertFalse(result['cleanup']['process_group_empty']);self.assertFalse(list(env.session.iterdir()))

 def test_ap3_batch_exception_preserves_final_companion_containment(self):
  env=types.SimpleNamespace(session=self.base/'batch',run_id='f'*32);env.session.mkdir()
  clean={'owned_descendants_zero':True,'process_group_empty':True};saved=[]
  def failed(*args,**kwargs):
   kwargs['checkpoint']('native_caller_retained',{'classification':'native_setup_failed','cleanup':clean,'caller':{'records':[]}},RuntimeError('setup timeout'))
   raise RuntimeError('setup timeout')
  with patch.object(self.profile,'core',side_effect=failed),patch.object(self.profile,'gui') as gui,patch.object(self.profile,'progress'):
   with self.assertRaisesRegex(RuntimeError,'setup timeout'):self.batch_supervise(env,mode=self.profile.MODE,profile=self.profile,checkpoint=lambda *v:saved.append(v))
  gui.assert_not_called();self.assertEqual(saved[-1][1]['cleanup'],clean)
  self.assertEqual(saved[-1][1]['current']['classification'],'native_setup_failed')

 def test_ap3_companion_retains_owned_descendants_and_report_on_failure(self):
  companion=self.profile.companion;environment=types.SimpleNamespace(session=self.base/'companion');environment.session.mkdir();(environment.session/'ap1.control').write_bytes(b'local substitute')
  process=types.SimpleNamespace(pid=123,poll=lambda:0)
  clean={'owned_descendants_zero':True,'process_group_empty':True};saved=[]
  def windows(*args,**kwargs):
   kwargs['observe_companion']();raise RuntimeError('original Windows supervision failure')
  with patch.object(companion.subprocess,'Popen',return_value=process),patch.object(companion.inherited,'process_identity',return_value={'pid':123,'start_ticks':10}),patch.object(companion.inherited,'descendants',return_value=[{'pid':124,'start_ticks':11}]),patch.object(companion.inherited,'cleanup_process',return_value=clean) as cleanup,patch.object(companion.diagnostic,'supervise',side_effect=windows):
   with self.assertRaisesRegex(RuntimeError,'original Windows supervision failure'):
    self.actual_profile_supervise(environment,mode='local',profile=self.profile,checkpoint=lambda *v:saved.append(v),caller_command=['substituted'],caller_env={'LOCAL':'true'},track_descendants=True,caller_report=lambda:b'{"event":"ap3_proxy_stats","fault":4}\n',accepted_events={'ap3_proxy_stats'})
  self.assertEqual(cleanup.call_args.args[1],[(123,10),(124,11)])
  retained=saved[-1][1]['caller'];self.assertEqual(retained['records'],[{'event':'ap3_proxy_stats','fault':4}]);self.assertEqual(retained['cleanup'],clean)

 def test_ap3_batch_reporting_rejection_never_launches_next_segment(self):
  env=types.SimpleNamespace(session=self.base/'batch',run_id='f'*32);env.session.mkdir()
  good=dict(classification='scanner_completed',cleanup={'owned_descendants_zero':True,'process_group_empty':True},records=[{'state':'original_observation'}]);saved=[]
  with patch.object(self.profile,'core',return_value=good) as run,patch.object(self.profile,'gui') as gui,patch.object(self.profile,'progress'),patch.object(self.profile,'normalize_session',side_effect=ValueError('retained caller failed')):
   with self.assertRaisesRegex(ValueError,'retained caller failed'):self.batch_supervise(env,mode=self.profile.MODE,profile=self.profile,checkpoint=lambda *v:saved.append(v))
  self.assertEqual(run.call_count,1);gui.assert_not_called();self.assertEqual(saved[-1][1]['segments']['core'],good);self.assertFalse(list(env.session.iterdir()))

def load_tests(loader,tests,pattern):return unittest.TestSuite(AP3ExecutionTests(n) for n in dir(AP3ExecutionTests) if n.startswith('test_ap3_'))

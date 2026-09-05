"""AP3 uses the actual classified worker; substitute only platform/process work."""
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
   return dict(records=[],raw_exit=1,classification='scanner_failed',cleanup={'owned_descendants_zero':True,'process_group_empty':True},caller={'records':[dict(event='ap3_host_error',stage='process',detail='original injected response fault',last_position=1024)],'raw_exit':1,'cleanup':{'owned_descendants_zero':True,'process_group_empty':True}})
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
  self.assertFalse(doc['binding']['acceptance_eligible']);self.assertFalse(self.rendered)
 def test_ap3_lost_ack_reconciles_without_replay(self):
  self.local_worker.lose_ack=True;self.execute_cli();self.execute_cli();self.assertEqual(self.scan.call_count,1)
 def test_ap3_read_only_preflight_does_not_run(self):
  result=self.execute_cli(['--preflight-only']);self.assertEqual(result['state'],'ready');self.assertEqual(self.scan.call_count,0)

def load_tests(loader,tests,pattern):return unittest.TestSuite(AP3ExecutionTests(n) for n in dir(AP3ExecutionTests) if n.startswith('test_ap3_'))

"""Actual classified worker/retention path; only platform and plug-in work substituted."""
import copy,json,pathlib,sys,types,unittest,importlib.util
from unittest.mock import patch
TOOLS=pathlib.Path(__file__).parent.resolve();sys.path.insert(0,str(TOOLS))
import ap2_adapter as a,ap2_contract as contract,pc0_proof_adapter as d
from test_ap1_execution import AP1ExecutionTests
from test_pc0_proof_adapter import SOURCE_A
from test_proof_cli import cli
from classified_proof_backend import ClassifiedProofBackend,AcceptancePlanAdapter,Observation
from proof_execution_policy import *

def host_records(reopen=False):
 from ap0_contract import bits,sample
 seed=2026
 records=[dict(event='ap2_host_ready',seed=seed,seed_after_activation=True,reopen=reopen)]
 total=0
 for b in range(1 if reopen else 13):
  if b==10:records.append(dict(event='ap2_host_rejections',count=10,audio_calls=10))
  if b==11:records.append(dict(event='ap2_host_flush',gain=.25,audio_calls=11))
  n=19 if reopen else contract.LENGTHS[b];gain=1. if reopen else contract.GAINS[b];flags=0 if reopen else contract.SILENCE[b]
  ins=contract.request_words(seed,b,n,flags);outs=[[bits(sample(w)*gain) for w in ch] for ch in ins]
  records.append(dict(event='ap2_host_block',block=b,frames=n,gain=gain,input_silence_flags=flags,output_silence_flags=3 if gain==0 or flags==3 else 0,result=0,in_place=not reopen and b==12,max_error=0.,comparison_ok=True,input_bits=ins,output_bits=outs));total+=n*2
 records.append(dict(event='ap2_host_compared',samples=total,audio_calls=1 if reopen else 13,max_error=0.))
 records.append(dict(event='ap2_host_closed',case='reopen' if reopen else 'positive',terminate_result=0,references_released=True,module_unloaded=True))
 return records

class AP2ExecutionTests(AP1ExecutionTests):
 def setUp(self):
  super().setUp();old=self.runtime
  native=dict(source_commit=SOURCE_A,input_sha256='1'*64,manifest_sha256='2'*64)
  plan=a.descriptor(ExecutionClass.ACCEPTANCE_CANDIDATE,old.artifact,native)
  r=a.AP2Runtime(old.ports,plan,old.artifact,native,repository=TOOLS.parent,proof_root=self.mac_proof);self.runtime=r;self.plan=plan
  self.stack.enter_context(patch.object(r,'_dependencies',old._dependencies));self.stack.enter_context(patch.object(r,'_local_preflight'))
  for name in ('ap2_native_artifact','ap2_contract','ap2_worker_support'):
   module=types.ModuleType(name);module.__file__='<'+name+'>';module._pc0_source_sha256=d.sha256_bytes(a.SUPPORT[name].encode())
   self.stack.enter_context(patch.dict(sys.modules,{name:module}));exec(compile(a.SUPPORT[name],module.__file__,'exec'),module.__dict__)
  self.profile=sys.modules['ap2_worker_support'];self.stack.enter_context(patch.object(self.profile,'verify_host',return_value=self.host));self.stack.enter_context(patch.object(self.profile,'bind_client'))
  oldscan=self.ap1_scan.side_effect
  def scan(*args,**kwargs):
   windows=[]
   template=copy.deepcopy(oldscan(*args,**kwargs));template.pop("caller",None)
   for iteration in range(2):
    result=copy.deepcopy(template);records=result['records'];records[:]=[r for r in records if r.get('state') not in {'ap0_process_started','ap0_process_completed','ap1_output_silence','ap1_private_buffers_valid'}]
    blocks=[v for v in host_records(iteration==1) if v['event']=='ap2_host_block'];extras=[]
    for b,block in enumerate(blocks):extras.extend([dict(event='lifecycle',state='ap0_process_started',block=b),dict(event='lifecycle',state='ap0_process_completed',block=b,result=0),dict(event='lifecycle',state='ap1_private_buffers_valid',block=b),dict(event='lifecycle',state='ap1_output_silence',block=b,input_silence_flags=block['input_silence_flags'],output_silence_flags=block['output_silence_flags'])])
    i=next(i for i,v in enumerate(records) if v.get('state')=='ap0_call_completed' and v.get('operation')=='set_processing_true');records[i+1:i+1]=extras
    for state,op,kind,seq in [('ap0_call_completed','set_active_true',9,1),('ap0_call_completed','set_processing_true',11,1),('ap0_thread_joined',None,13,len(blocks)+1),('ap0_call_completed','deactivate_audio_input',15,len(blocks)+1)]:
     i=next(i for i,v in enumerate(records) if v.get('state')==state and v.get('operation')==op)
     records.insert(i+1,dict(event='lifecycle',state='ap2_lifecycle_ack',kind=kind,next_sequence=seq))
    records.insert(1,dict(event='lifecycle',state='ap2_processor_traits',latency_samples=0,tail_samples=0))
    mapping={v['sequence']:i+1 for i,v in enumerate(records) if 'sequence' in v}
    for i,v in enumerate(records):
     for key in ('attempt_sequence','enclosing_attempt_sequence'):
      if v.get(key) in mapping:v[key]=mapping[v[key]]
     v['sequence']=i+1
    windows.append(result)
   return {'records':[],'classification':'scanner_completed','cleanup':windows[-1]['cleanup'],'activations':windows,'caller':{'raw_exit':0,'cleanup':{'owned_descendants_zero':True,'process_group_empty':True},'records':host_records()+host_records(True)}}
  self.ap2_scan=self.stack.enter_context(patch.object(self.profile,'supervise',side_effect=scan))
  self.adapter=AcceptancePlanAdapter(descriptor=plan,preflight=r.preflight,reconcile=r.reconcile,invoke=r.invoke_diagnostic,admit=r.admit,render_product_evidence=lambda c,v:self.render(c,v))
  self.backend=ClassifiedProofBackend(self.base/'state',{plan.plan_id:self.adapter})
  fields=dict(self.auth.fields);fields.update(authorized_plan_id=plan.plan_id,authorized_product_contract_identity=plan.product_contract_identity,authorized_product_contract_sha256=plan.product_contract_sha256,authorized_plan_content_sha256=plan.plan_content_sha256,acceptance_candidate_identity=a.candidate_identity(SOURCE_A,plan))
  self.authority.write_text('## Authority\n```yaml\n'+'\n'.join(f'{k}: {v}' for k,v in fields.items())+'\n```\n');self.auth=load_authority(self.authority)
  self.delegation=authorize_live_request(self.auth,LiveRequest(ExecutionClass.ACCEPTANCE_CANDIDATE,SOURCE_A,plan.plan_id,fields['acceptance_candidate_identity']));self.current_reservation=self.backend._reservation_identity(self.delegation)
  self.args=['accept','--authority',str(self.authority),'--source',SOURCE_A,'--plan',plan.plan_id,'--candidate',fields['acceptance_candidate_identity']]
  self.stack.enter_context(patch.object(cli,'_production_adapters',return_value=({plan.plan_id:self.adapter},types.SimpleNamespace(last_failure=None))))
 def render(self,context,result):
  self.runtime.admit(context,Observation.from_record(result['observation']));contract.validate_summary(result['admitted_result']['document']['summary']);self.rendered.append(result)
 def test_ap2_fresh_candidate_compares_host_and_reopen(self):
  self.assertEqual(self.execute_cli(['--preflight-only'])['state'],'ready');self.assertEqual(self.launches,0)
  receipt=self.execute_cli();self.assertTrue(receipt['renderer_completed'],self.transaction())
  s=self.rendered[0]['admitted_result']['document']['summary'];self.assertEqual(s['comparison']['samples_compared'],1730);self.assertEqual(len(s['activations']),2)
 def test_ap2_lost_ack_never_reexecutes_batch(self):
  self.local_worker.lose_ack=True;self.execute_cli();self.execute_cli();self.assertEqual(self.ap2_scan.call_count,1)
 def test_ap2_normalizer_failure_retains_host_and_both_windows_sessions(self):
  with patch.object(self.profile,'normalize',side_effect=ValueError('original normalizer failure')):self.execute_cli()
  self.assertFalse(self.rendered);self.assertEqual(self.retire.call_count,1)
  doc=self.transaction()['observation']['payload']['document'];available=doc['summary']['troubleshooting']['observation']
  self.assertEqual(available['caller']['records'][1]['output_bits'],host_records()[1]['output_bits']);self.assertEqual(len(available['activations']),2)
 def test_ap2_build_surface_is_explicit_and_still_rejects_state_calls(self):
  import tempfile,shutil
  spec=importlib.util.spec_from_file_location('ap2_build_verify',TOOLS/'wf0-factory-census/verify.py');module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
  # This is the retired AP4 producer's closed-surface contract. AP8 has
  # additional SDK capabilities and uses its own build; keep this historical
  # verifier strict and exercise it against the accepted AP4 implementation.
  import subprocess
  with tempfile.TemporaryDirectory() as temp:
   target=pathlib.Path(temp)
   basis='93a00ed'
   paths=subprocess.check_output(['git','ls-tree','-r','--name-only',basis,'windows-factory-probe'],cwd=TOOLS.parent,text=True).splitlines()
   for relative in paths:
    path=target/relative;path.parent.mkdir(parents=True,exist_ok=True)
    path.write_bytes(subprocess.check_output(['git','show',basis+':'+relative],cwd=TOOLS.parent))
   with self.assertRaises(Exception):module.scanner_component_call_surface(target,ap0=True,ap2=True)
   self.assertEqual(module.scanner_component_call_surface(target,ap0=True,ap2=True,ap4=True)['closed_plugin_operation_count'],21)
   with self.assertRaises(Exception):module.scanner_component_call_surface(target,ap0=True)
   path=target/'windows-factory-probe/source/main.cpp';path.write_text(path.read_text()+'\ncomponent->setState(nullptr);\n')
   with self.assertRaises(Exception):module.scanner_component_call_surface(target,ap0=True,ap2=True,ap4=True)
 def test_ap2_host_output_corruption_rejected(self):
  records=host_records();records[1]['output_bits'][0][0]=0
  with self.assertRaises(ValueError):contract.compare(records)
 def test_ap2_reporting_failure_keeps_original_audio(self):
  with patch.object(self.profile,'validate_summary',side_effect=ValueError('report rejection')):self.execute_cli()
  self.assertFalse(self.rendered);doc=self.transaction()['observation']['payload']['document'];self.assertEqual(doc['cleanup'],'COMPLETE')
  self.assertIn('report rejection',json.dumps(doc));self.assertEqual(len(doc['summary']['troubleshooting']['observation']['activations']),2)

def load_tests(loader,tests,pattern):return unittest.TestSuite(AP2ExecutionTests(n) for n in dir(AP2ExecutionTests) if n.startswith('test_ap2_'))

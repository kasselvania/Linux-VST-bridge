"""AP2 plans on the existing classified execution, custody and retention path."""
import importlib.util,json,pathlib,os,pwd,sys
import ap0_adapter as base
import ap1_adapter as previous
import ap1_runtime as runtime
import pc0_proof_adapter as d
from ap2_contract import validate_summary
from ap2_native_artifact import PATHS,canonical,digest,verify_native
from classified_proof_backend import PlanDescriptor,AcceptancePlanAdapter,DiagnosticPlanAdapter,Observation
from proof_execution_policy import ExecutionClass,canonical_json,sha256_bytes
ROOT=pathlib.Path(__file__).resolve().parents[1]
CONTRACT='docs/slices/AP2/CONTRACT.md';ARTIFACT='docs/campaigns/AP2_ARTIFACT.json';NATIVE='docs/campaigns/AP2_NATIVE.json'
SUPPORT={**previous.SUPPORT,**{n:(ROOT/'tools'/f'{n}.py').read_text() for n in ('ap2_native_artifact','ap2_contract','ap2_worker_support')}}
PROGRAM=base.WORKER.replace('_SUPPORT_SOURCES = {}','_SUPPORT_SOURCES = '+repr(SUPPORT))
def candidate_identity(source,plan):
 return sha256_bytes(canonical_json({'schema':'ap2-candidate/v1','source':source,'plan':plan.record()}))
def descriptor(cls,artifact,native):
 return PlanDescriptor.create(plan_id='ap2-native-vst3-offline-'+('acceptance' if cls is ExecutionClass.ACCEPTANCE_CANDIDATE else 'diagnostic')+'-v1',
  execution_class=cls,product_contract_identity='ap2-native-vst3-offline-v1',product_contract_bytes=(ROOT/CONTRACT).read_bytes(),operation='ap2-sdk-host-process-and-reopen',artifact_requirement={**artifact,'native_client':native},
  fixture_requirement=d.FIXTURE_REQUIREMENT,runtime_requirement={**d.RUNTIME_REQUIREMENT,'runtime_proton_identity_sha256':runtime.identity(),'declared_runtime_inputs_sha256':runtime.declared_inputs()})
def native_parent():
 return pathlib.Path(pwd.getpwuid(os.getuid()).pw_dir)/'Library/Application Support/Linux VST Bridge/proof/native-artifacts/by-manifest'
class AP2Runtime(base.AP0Runtime):
 product='AP2';diagnostic_budget=10;windows_branch='codex/ap2-native-vst3-offline-bridge';windows_input_count=22
 candidate_key=staticmethod(candidate_identity);check_summary=staticmethod(validate_summary);validate_runtime=staticmethod(runtime.validate_observation)
 support_sources=SUPPORT;remote_program=PROGRAM;worker_sha256=sha256_bytes(PROGRAM.encode())
 def __init__(self,ports,plan,artifact,native,**kwargs):
  super().__init__(ports,plan,artifact,**kwargs);self.native=native
 def _dependencies(self,source):
  dep=super()._dependencies(source)
  paths={r['path'] for r in dep['candidate']['records']}|set(PATHS)|{CONTRACT,ARTIFACT,NATIVE,'tools/ap2_adapter.py','tools/ap2_contract.py','tools/ap2_native_artifact.py','tools/ap2_worker_support.py','tools/ap1_adapter.py','tools/ap1_client_artifact.py','tools/ap1_contract.py','tools/ap1_worker_support.py','tools/ap1_runtime.py'}
  dep['candidate']['records']=[{'path':p,'git_blob':d._checked_text(self.ports.command.run(('git','rev-parse',source+':'+p),cwd=self.repository),'AP2 source input')} for p in sorted(paths)]
  return dep
 def _request(self,delegation,reservation):
  return {**super()._request(delegation,reservation),'native_client':self.native,'runtime_identity':runtime.identity(),'declared_runtime_inputs_sha256':runtime.declared_inputs()}
 def _local_preflight(self,delegation):
  super()._local_preflight(delegation)
  record=verify_native(native_parent()/self.native['manifest_sha256'],self.native,self.product in {'AP3','AP4'},self.product=='AP4')
  inputs=[{'path':p,'git_blob':d._checked_text(self.ports.command.run(('git','rev-parse',delegation['source_commit']+':'+p),cwd=self.repository),'AP2 native source')} for p in PATHS]
  if record['records']!=inputs or digest(canonical(inputs))!=self.native['input_sha256']:raise d.AdapterBoundaryError('AP2 native source differs from retained build')
 def render(self,context,result):
  if self.admit(context,Observation.from_record(result['observation']))!=result['admitted_result'] or not self.acceptance:raise d.AdapterBoundaryError('AP2 renderer requires fresh acceptance')
  sys.path.insert(0,str(self.repository/'tools/wf0-factory-census'))
  spec=importlib.util.spec_from_file_location('ap2_evidence',self.repository/'tools/wf0-factory-census/evidence.py');module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
  s=validate_summary(result['admitted_result']['document']['summary'])
  module.render_ap0_packet(self.repository/'evidence/ap2-native-vst3-offline-bridge',result,consumer_source=d._checked_text(self.ports.command.run(('git','rev-parse','HEAD'),cwd=self.repository),'AP2 consumer'),validate=validate_summary,product='AP2',findings=[
   f"The SDK host independently checked {s['comparison']['samples_compared']} actual returned float32 samples; maximum absolute error {s['comparison']['maximum_absolute_error']}.",
   'Standard VST3 factory/component/processor interfaces only. Thirteen blocks in one persistent activation cover gain and length changes, zero gain, whole/one-channel silence, empty-queue gain persistence, zero-frame parameter flush and in-place buffers. Fresh module reopen checks default gain through another real Windows instance.',
   'Windows owner/processing-thread transitions acknowledged the host. Both sessions stopped, joined, deactivated, terminated and unloaded; both mappings retired. Host references released; owned groups empty; disposable stage absent; protected state unchanged.',
   'Offline processor-only bridge. Realtime/prefetch, 64-bit audio, unsupported automation/events, controller/editor and state persistence are not supported. AP1 remains the accepted frontier pending AP2 review and merge.'])
def adapters(ports,repository=ROOT):
 if not (repository/ARTIFACT).exists() or not (repository/NATIVE).exists():return {}
 artifact=json.loads((repository/ARTIFACT).read_bytes());native=json.loads((repository/NATIVE).read_bytes())
 if set(artifact)!={'producer_source','windows_build_input_identity','producer_run_id','producer_run_attempt','artifact_id','host_manifest_sha256'} or set(native)!={'source_commit','input_sha256','manifest_sha256'}:raise d.AdapterBoundaryError('AP2 artifact roster differs')
 registry={}
 for cls in (ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE,ExecutionClass.ACCEPTANCE_CANDIDATE):
  plan=descriptor(cls,artifact,native);r=AP2Runtime(ports,plan,artifact,native,repository=repository)
  args=dict(descriptor=plan,preflight=r.preflight,reconcile=r.reconcile,invoke=r.invoke_diagnostic,admit=r.admit)
  registry[plan.plan_id]=AcceptancePlanAdapter(**args,render_product_evidence=r.render) if r.acceptance else DiagnosticPlanAdapter(**args)
 return registry

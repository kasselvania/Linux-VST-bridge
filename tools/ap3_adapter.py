"""Closed AP3 plans reuse the classified AP2/companion execution machinery."""
import json,sys,importlib.util
import ap2_adapter as previous
import ap0_adapter as base
import ap1_runtime as runtime
import pc0_proof_adapter as d
from ap3_contract import validate_summary
from classified_proof_backend import PlanDescriptor,DiagnosticPlanAdapter,AcceptancePlanAdapter
from proof_execution_policy import ExecutionClass,canonical_json,sha256_bytes
ROOT=previous.ROOT
CONTRACT='docs/slices/AP3/CONTRACT.md';ARTIFACT='docs/campaigns/AP3_ARTIFACT.json';NATIVE='docs/campaigns/AP3_NATIVE.json'
SUPPORT={**previous.SUPPORT,**{n:(ROOT/'tools'/f'{n}.py').read_text() for n in ('ap3_contract','ap3_worker_support')}}
PROGRAM=base.WORKER.replace('_SUPPORT_SOURCES = {}','_SUPPORT_SOURCES = '+repr(SUPPORT))
def candidate_identity(source,plan):
 return sha256_bytes(canonical_json({'schema':'ap3-candidate/v1','source':source,'plan':plan.record()}))
def descriptor(cls,artifact,native):
 return PlanDescriptor.create(plan_id='ap3-sustained-audio-'+('acceptance' if cls is ExecutionClass.ACCEPTANCE_CANDIDATE else 'diagnostic')+'-v1',
  execution_class=cls,product_contract_identity='ap3-sustained-audio-v1',product_contract_bytes=(ROOT/CONTRACT).read_bytes(),operation='ap3-paced-streams-and-bitwig-reopen',artifact_requirement={**artifact,'native_client':native},
  fixture_requirement=d.FIXTURE_REQUIREMENT,runtime_requirement={**d.RUNTIME_REQUIREMENT,'runtime_proton_identity_sha256':runtime.identity(),'declared_runtime_inputs_sha256':runtime.declared_inputs()})
class AP3Runtime(previous.AP2Runtime):
 product='AP3';diagnostic_budget=10;windows_branch='codex/ap3-sustained-audio-and-remote-desktop';windows_input_count=22
 candidate_key=staticmethod(candidate_identity);check_summary=staticmethod(validate_summary)
 support_sources=SUPPORT;remote_program=PROGRAM;worker_sha256=sha256_bytes(PROGRAM.encode())
 def _dependencies(self,source):
  dep=super()._dependencies(source)
  paths={r['path'] for r in dep['candidate']['records']}|{CONTRACT,ARTIFACT,NATIVE,'tools/ap3_adapter.py','tools/ap3_contract.py','tools/ap3_worker_support.py'}
  dep['candidate']['records']=[{'path':p,'git_blob':d._checked_text(self.ports.command.run(('git','rev-parse',source+':'+p),cwd=self.repository),'AP3 source input')} for p in sorted(paths)]
  return dep
 def render(self,context,result):
  from classified_proof_backend import Observation
  if self.admit(context,Observation.from_record(result['observation']))!=result['admitted_result'] or not self.acceptance:raise d.AdapterBoundaryError('AP3 fresh acceptance required')
  summary=validate_summary(result['admitted_result']['document']['summary'])
  if len(summary.get('bitwig',[]))!=2 or 'stream_active' not in summary:raise d.AdapterBoundaryError('AP3 complete same-build DAW and stream-active batch required')
  sys.path.insert(0,str(self.repository/'tools/wf0-factory-census'))
  spec=importlib.util.spec_from_file_location('ap3_evidence',self.repository/'tools/wf0-factory-census/evidence.py');module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
  module.render_ap0_packet(self.repository/'evidence/ap3-sustained-native-audio',result,consumer_source=d._checked_text(self.ports.command.run(('git','rev-parse','HEAD'),cwd=self.repository),'AP3 consumer'),validate=validate_summary,product='AP3',findings=[
   f"The SDK-loaded proxy returned {summary['comparison']['samples_compared']} independently reconstructed and checked samples at 1024 samples added latency; maximum error {summary['comparison']['maximum_absolute_error']}.",
   'Thirty-second 128/256-frame primary streams used no active desktop stream. The separately labelled short stream-active check returned 602048 additional checked samples. Callback instrumentation, per-stream deadlines and queue faults were admitted independently.',
   'Two disposable Bitwig sessions used this exact native artifact and actual Windows AGain. Native termination counters and Windows lifecycle records corroborate the separately retained agent GUI observations. Streaming and meters are not numerical or timing oracles.',
   'All owned groups empty, ordinary plug-in shutdown, mappings closed, disposable stage absent and protected state unchanged. AP2 remains accepted pending AP3 review. Reference-effect preview only; no general realtime, vendor state/editor or commercial compatibility claim.'])

def adapters(ports,repository=ROOT):
 if not (repository/ARTIFACT).exists() or not (repository/NATIVE).exists():return {}
 artifact=json.loads((repository/ARTIFACT).read_bytes());native=json.loads((repository/NATIVE).read_bytes())
 if set(artifact)!={'producer_source','windows_build_input_identity','producer_run_id','producer_run_attempt','artifact_id','host_manifest_sha256'} or set(native)!={'source_commit','input_sha256','manifest_sha256'}:raise d.AdapterBoundaryError('AP3 artifact roster differs')
 registry={}
 for cls in (ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE,ExecutionClass.ACCEPTANCE_CANDIDATE):
  plan=descriptor(cls,artifact,native);r=AP3Runtime(ports,plan,artifact,native,repository=repository)
  args=dict(descriptor=plan,preflight=r.preflight,reconcile=r.reconcile,invoke=r.invoke_diagnostic,admit=r.admit)
  registry[plan.plan_id]=AcceptancePlanAdapter(**args,render_product_evidence=r.render) if r.acceptance else DiagnosticPlanAdapter(**args)
 return registry

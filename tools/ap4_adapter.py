"""Scoped AP4 plans on the existing classified production backend."""
import json,sys,importlib.util
import ap2_adapter as previous
import ap3_adapter as ap3
import ap0_adapter as base
import ap1_runtime as runtime
import pc0_proof_adapter as d
from ap4_contract import validate_summary,GUI
from classified_proof_backend import PlanDescriptor,DiagnosticPlanAdapter,AcceptancePlanAdapter,Observation
from proof_execution_policy import ExecutionClass,canonical_json,sha256_bytes
ROOT=previous.ROOT
CONTRACT='CURRENT_SLICE.md';ARTIFACT='docs/campaigns/AP4_ARTIFACT.json';NATIVE='docs/campaigns/AP4_NATIVE.json'
FULL_DIAGNOSTIC=True
SUPPORT={**ap3.SUPPORT,**{n:(ROOT/'tools'/f'{n}.py').read_text() for n in ('ap4_contract','ap4_worker_support')}}
PROGRAM=base.WORKER.replace('_SUPPORT_SOURCES = {}','_SUPPORT_SOURCES = '+repr(SUPPORT))
def candidate_identity(source,plan):return sha256_bytes(canonical_json({'schema':'ap4-candidate/v1','source':source,'plan':plan.record()}))
def descriptor(cls,artifact,native):
 full=cls is ExecutionClass.ACCEPTANCE_CANDIDATE or FULL_DIAGNOSTIC
 return PlanDescriptor.create(plan_id='ap4-state-recall-'+('acceptance' if cls is ExecutionClass.ACCEPTANCE_CANDIDATE else 'diagnostic')+'-v1',execution_class=cls,
  product_contract_identity='ap4-plugin-state-recall-v1',product_contract_bytes=(ROOT/CONTRACT).read_bytes(),operation='ap4-state-three-sdk-and-three-bitwig' if full else 'ap4-state-three-sdk',
  artifact_requirement={**artifact,'native_client':native},fixture_requirement=d.FIXTURE_REQUIREMENT,
  runtime_requirement={**d.RUNTIME_REQUIREMENT,'runtime_proton_identity_sha256':runtime.identity(),'declared_runtime_inputs_sha256':runtime.declared_inputs()})
class AP4Runtime(previous.AP2Runtime):
 product='AP4';diagnostic_budget=10;windows_branch='codex/ap4-plugin-state-project-recall';windows_input_count=23
 candidate_key=staticmethod(candidate_identity);check_summary=staticmethod(validate_summary)
 support_sources=SUPPORT;remote_program=PROGRAM;worker_sha256=sha256_bytes(PROGRAM.encode())
 def _dependencies(self,source):
  dep=super()._dependencies(source)
  paths={r['path'] for r in dep['candidate']['records']}|{CONTRACT,ARTIFACT,NATIVE,'tools/ap3_adapter.py','tools/ap3_contract.py','tools/ap3_worker_support.py','tools/ap4_adapter.py','tools/ap4_contract.py','tools/ap4_worker_support.py'}
  dep['candidate']['records']=[{'path':p,'git_blob':d._checked_text(self.ports.command.run(('git','rev-parse',source+':'+p),cwd=self.repository),'AP4 source input')} for p in sorted(paths)];return dep
 def _request(self,delegation,reservation):return {**super()._request(delegation,reservation),'ap4_gui':self.acceptance or FULL_DIAGNOSTIC}
 def render(self,context,result):
  if self.admit(context,Observation.from_record(result['observation']))!=result['admitted_result'] or not self.acceptance:raise d.AdapterBoundaryError('fresh AP4 acceptance required')
  summary=validate_summary(result['admitted_result']['document']['summary'])
  if not set(GUI)<=set(summary['sessions']):raise d.AdapterBoundaryError('AP4 saved project recall absent')
  sys.path.insert(0,str(self.repository/'tools/wf0-factory-census'))
  spec=importlib.util.spec_from_file_location('ap4_evidence',self.repository/'tools/wf0-factory-census/evidence.py');module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
  module.render_ap0_packet(self.repository/'evidence/ap4-plugin-state-project-recall',result,consumer_source=d._checked_text(self.ports.command.run(('git','rev-parse','HEAD'),cwd=self.repository),'AP4 consumer'),validate=validate_summary,product='AP4',findings=[
   f"Real Windows component state preserved all 12 bytes (gain, reduction, bypass). Fresh sessions restored 0.25 and mute without a driver gain resend. Independently checked samples: {summary['comparison']['samples_compared']}; maximum error {summary['comparison']['maximum_absolute_error']}.",
   'A fresh 30-second paced stream included coherent saves and zero-frame updates with zero callback effects, overruns or queue faults. Hidden reduction and bypass states were restored through standard VST3 methods and changed actual returned samples.',
   'The disposable Bitwig project was saved at nondefault gain, closed with every owned process, reopened and observed before editing, saved at mute, then fully closed/reopened again. Actual component calls, state digests, independent sample comparisons and GUI observations corroborate recall. No sidecar or driver restored gain.',
   'All owned sessions/mappings retired and protected state unchanged. AP3 remains the accepted frontier pending review. This is the retained reference fixture only; no general vendor state/editor or commercial compatibility claim.'])
def adapters(ports,repository=ROOT):
 if not (repository/ARTIFACT).exists() or not (repository/NATIVE).exists():return {}
 artifact=json.loads((repository/ARTIFACT).read_bytes());native=json.loads((repository/NATIVE).read_bytes())
 if set(artifact)!={'producer_source','windows_build_input_identity','producer_run_id','producer_run_attempt','artifact_id','host_manifest_sha256'} or set(native)!={'source_commit','input_sha256','manifest_sha256'}:raise d.AdapterBoundaryError('AP4 artifact roster differs')
 registry={}
 for cls in (ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE,ExecutionClass.ACCEPTANCE_CANDIDATE):
  plan=descriptor(cls,artifact,native);r=AP4Runtime(ports,plan,artifact,native,repository=repository)
  args=dict(descriptor=plan,preflight=r.preflight,reconcile=r.reconcile,invoke=r.invoke_diagnostic,admit=r.admit)
  registry[plan.plan_id]=AcceptancePlanAdapter(**args,render_product_evidence=r.render) if r.acceptance else DiagnosticPlanAdapter(**args)
 return registry

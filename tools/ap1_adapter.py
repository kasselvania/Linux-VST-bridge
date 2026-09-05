"""AP1 plans reuse the classified AP0 transaction and supervision path."""
import importlib.util,json,pathlib,pwd,os,sys
import ap0_adapter as base
import pc0_proof_adapter as d
from ap1_contract import validate_summary
from ap1_client_artifact import PATHS,canonical,digest,verify_client
from classified_proof_backend import PlanDescriptor,AcceptancePlanAdapter,DiagnosticPlanAdapter,Observation
from proof_execution_policy import ExecutionClass,canonical_json,sha256_bytes
ROOT=pathlib.Path(__file__).resolve().parents[1]
CONTRACT='docs/slices/AP1/CONTRACT.md';ARTIFACT='docs/campaigns/AP1_ARTIFACT.json';CLIENT='docs/campaigns/AP1_CLIENT.json'
EVIDENCE='evidence/ap1-linux-windows-audio-roundtrip-a2'
SUPPORT={**base.SUPPORT,**{n:(ROOT/'tools'/f'{n}.py').read_text() for n in ('ap1_client_artifact','ap1_contract','ap1_worker_support')}}
# The existing entrypoint has no product policy or execution implementation.
PROGRAM=base.WORKER.replace('_SUPPORT_SOURCES = {}','_SUPPORT_SOURCES = '+repr(SUPPORT))
def candidate_identity(source,plan):
    return sha256_bytes(canonical_json({'schema':'ap1-candidate/v1','source':source,'plan':plan.record()}))
def descriptor(cls,artifact,client):
    return PlanDescriptor.create(plan_id='ap1-linux-windows-audio-'+('acceptance' if cls is ExecutionClass.ACCEPTANCE_CANDIDATE else 'diagnostic')+'-v1',
        execution_class=cls,product_contract_identity='ap1-linux-windows-audio-v1',product_contract_bytes=(ROOT/CONTRACT).read_bytes(),
        operation='ap1-mapped-stereo-ten-blocks',artifact_requirement={**artifact,'native_client':client},
        fixture_requirement=d.FIXTURE_REQUIREMENT,runtime_requirement={**d.RUNTIME_REQUIREMENT,'declared_runtime_inputs_sha256':base.declared_runtime_inputs()})
def client_parent():
    return pathlib.Path(pwd.getpwuid(os.getuid()).pw_dir)/'Library/Application Support/Linux VST Bridge/proof/client-artifacts/by-manifest'
class AP1Runtime(base.AP0Runtime):
    product='AP1';diagnostic_budget=10;windows_branch='codex/ap1-linux-windows-audio-roundtrip';windows_input_count=22
    candidate_key=staticmethod(candidate_identity);check_summary=staticmethod(validate_summary)
    support_sources=SUPPORT;remote_program=PROGRAM;worker_sha256=sha256_bytes(PROGRAM.encode())
    def __init__(self,ports,plan,artifact,client,**kwargs):
        super().__init__(ports,plan,artifact,**kwargs);self.client=client
    def _dependencies(self,source):
        dep=super()._dependencies(source)
        paths={r['path'] for r in dep['candidate']['records']}|set(PATHS)|{CONTRACT,ARTIFACT,CLIENT,'tools/ap1_adapter.py','tools/ap1_contract.py','tools/ap1_client_artifact.py','tools/ap1_build_client.py','tools/ap1_worker_support.py'}
        dep['candidate']['records']=[{'path':p,'git_blob':d._checked_text(self.ports.command.run(('git','rev-parse',source+':'+p),cwd=self.repository),'AP1 source input')} for p in sorted(paths)]
        return dep
    def _request(self,delegation,reservation):
        return {**super()._request(delegation,reservation),'native_client':self.client}
    def _local_preflight(self,delegation):
        super()._local_preflight(delegation)
        record=verify_client(client_parent()/self.client['manifest_sha256'],self.client)
        inputs=[{'path':p,'git_blob':d._checked_text(self.ports.command.run(('git','rev-parse',delegation['source_commit']+':'+p),cwd=self.repository),'AP1 native source')} for p in PATHS]
        if record['records']!=inputs or digest(canonical(inputs))!=self.client['input_sha256']:
            raise d.AdapterBoundaryError('AP1 native source differs from retained build')
    def render(self,context,result):
        if self.admit(context,Observation.from_record(result['observation']))!=result['admitted_result'] or not self.acceptance:
            raise d.AdapterBoundaryError('AP1 renderer requires fresh acceptance')
        sys.path.insert(0,str(self.repository/'tools/wf0-factory-census'))
        spec=importlib.util.spec_from_file_location('ap1_evidence',self.repository/'tools/wf0-factory-census/evidence.py')
        module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
        module.render_ap1_packet(self.repository/EVIDENCE,result,consumer_source=d._checked_text(self.ports.command.run(('git','rev-parse','HEAD'),cwd=self.repository),'AP1 consumer'),validate=validate_summary)
def adapters(ports,repository=ROOT):
    if not (repository/ARTIFACT).exists() or not (repository/CLIENT).exists():return {}
    artifact=json.loads((repository/ARTIFACT).read_bytes());client=json.loads((repository/CLIENT).read_bytes())
    if set(artifact)!={'producer_source','windows_build_input_identity','producer_run_id','producer_run_attempt','artifact_id','host_manifest_sha256'} or set(client)!={'source_commit','input_sha256','binary_sha256','manifest_sha256'}:
        raise d.AdapterBoundaryError('AP1 artifact roster differs')
    registry={}
    for cls in (ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE,ExecutionClass.ACCEPTANCE_CANDIDATE):
        plan=descriptor(cls,artifact,client);r=AP1Runtime(ports,plan,artifact,client,repository=repository)
        args=dict(descriptor=plan,preflight=r.preflight,reconcile=r.reconcile,invoke=r.invoke_diagnostic,admit=r.admit)
        registry[plan.plan_id]=AcceptancePlanAdapter(**args,render_product_evidence=r.render) if r.acceptance else DiagnosticPlanAdapter(**args)
    return registry

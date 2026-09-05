"""Thin AP0 plans over the existing classified backend, transport and worker."""
from __future__ import annotations
import json
import pathlib
import sys
import importlib.util
import pc0_proof_adapter as d
import pc0_acceptance_adapter as pc0
from ap0_contract import validate_summary
from pc0_contract import acceptance_execution_input_sha256
from pc0_diagnostic_runtime import validate_runtime_observation, declared_runtime_inputs, exception_detail, checkpoint_projection
from classified_proof_backend import AcceptancePlanAdapter, DiagnosticPlanAdapter, Observation, ObservationKind, PlanDescriptor
from proof_execution_policy import ExecutionClass, canonical_json, sha256_bytes

ROOT=pathlib.Path(__file__).resolve().parents[1]
CONTRACT='docs/slices/AP0/CONTRACT.md'
ARTIFACT='docs/campaigns/AP0_ARTIFACT.json'
EVIDENCE='evidence/ap0-offline-again-processing'
SUPPORT={**d.SUPPORT_SOURCES,'pc0_contract':(ROOT/'tools/pc0_contract.py').read_text(),
         'ap0_contract':(ROOT/'tools/ap0_contract.py').read_text(),
         'ap0_artifacts':(ROOT/'tools/wf0-factory-census/artifacts.py').read_text(),
         'ap0_worker_support':(ROOT/'tools/ap0_worker_support.py').read_text()}
WORKER=(ROOT/'tools/ap0_worker.py').read_text()
PROGRAM=WORKER.replace('_SUPPORT_SOURCES = {}','_SUPPORT_SOURCES = '+repr(SUPPORT))
WORKER_SHA=sha256_bytes(PROGRAM.encode())


def candidate_identity(source,descriptor):
    return sha256_bytes(canonical_json({'schema':'ap0-candidate/v1','source':source,'plan':descriptor.record()}))

def descriptor(execution_class,artifact):
    return PlanDescriptor.create(plan_id='ap0-offline-again-'+('acceptance' if execution_class is ExecutionClass.ACCEPTANCE_CANDIDATE else 'diagnostic')+'-v1',
        execution_class=execution_class, product_contract_identity='ap0-offline-again-v1',
        product_contract_bytes=(ROOT/CONTRACT).read_bytes(),operation='ap0-offline-stereo-three-blocks',
        artifact_requirement=artifact,fixture_requirement=d.FIXTURE_REQUIREMENT,
        runtime_requirement={**d.RUNTIME_REQUIREMENT,'declared_runtime_inputs_sha256':declared_runtime_inputs()})

class AP0Runtime(d.PC0DiagnosticRuntime):
    product='AP0'
    candidate_key=staticmethod(candidate_identity)
    diagnostic_budget=8
    windows_branch='codex/ap0-offline-again-processing'
    windows_input_count=19
    check_summary=staticmethod(validate_summary)
    validate_runtime=staticmethod(validate_runtime_observation)
    worker_path='tools/ap0_worker.py'
    worker_source=WORKER
    support_sources=SUPPORT
    worker_sha256=WORKER_SHA
    remote_program=PROGRAM
    def __init__(self,ports,plan,artifact,**kwargs):
        super().__init__(ports,**kwargs)
        self.plan=plan;self.artifact=artifact
        self.acceptance=plan.execution_class is ExecutionClass.ACCEPTANCE_CANDIDATE
        self.publication_effect='acceptance_publications' if self.acceptance else 'diagnostic_publications'
    def _validate_context(self,value,plan):
        v=dict(value)
        if (dict(plan)!=self.plan.record() or v['execution_class']!=self.plan.execution_class.value
                or v['acceptance_eligible'] is not self.acceptance
                or v['batch_budget_maximum']!=(1 if self.acceptance else self.diagnostic_budget)
                or v['plan_content_sha256']!=self.plan.plan_content_sha256
                or v['product_contract_identity']!=self.plan.product_contract_identity
                or v['product_contract_sha256']!=self.plan.product_contract_sha256
                or v['plan_id']!=self.plan.plan_id
                or (self.acceptance and v['execution_identity']!=self.candidate_key(v['source_commit'],self.plan))):
            raise d.AdapterBoundaryError('AP0 authority/plan/candidate differs')
        return v
    def _dependencies(self,source):
        dep=pc0.PC0AcceptanceRuntime._dependencies(self,source)
        paths={r['path'] for r in dep['candidate']['records']} | {
            CONTRACT,ARTIFACT,'tools/ap0_adapter.py','tools/ap0_worker.py',
            'tools/ap0_worker_support.py','tools/ap0_contract.py','tools/wf0-factory-census/artifacts.py'}
        dep['candidate']['records']=[{'path':p,'git_blob':d._checked_text(self.ports.command.run(
            ('git','rev-parse',source+':'+p),cwd=self.repository),'AP0 source input')} for p in sorted(paths)]
        return dep
    def _request(self,delegation,reservation):
        r=super()._request(delegation,reservation)
        r.update(product=self.product,acceptance_eligible=self.acceptance,
            plan_content_sha256=self.plan.plan_content_sha256,
            declared_runtime_inputs_sha256=declared_runtime_inputs(),
            execution_dependencies=self._dependencies(delegation['source_commit']),
            **self.artifact)
        if self.acceptance:r['candidate_identity']=r.pop('campaign_identity')
        return r
    def _local_preflight(self,delegation):
        source=delegation['source_commit']
        head=d._checked_text(self.ports.command.run(('git','rev-parse','HEAD'),cwd=self.repository),'AP0 HEAD')
        clean=self.ports.command.run(('git','status','--porcelain=v1','--untracked-files=all'),cwd=self.repository)
        if head!=source or clean.returncode or clean.stdout or clean.stderr:
            raise d.AdapterBoundaryError('AP0 source checkout differs')
        for record in self._dependencies(source)['candidate']['records']:
            if d._git_blob_sha1(self.ports.filesystem.read_bytes(self.repository/record['path'],2*1024*1024))!=record['git_blob']:
                raise d.AdapterBoundaryError('AP0 dependency differs: '+record['path'])
        sys.path.insert(0,str(self.repository/'tools/wf0-factory-census'))
        import artifacts,common
        if common.dx0_identity_sha256(getattr(common,self.product.lower()+'_windows_build_input')(source))!=self.artifact['windows_build_input_identity']:
            raise d.AdapterBoundaryError('AP0 source Windows inputs differ from retained producer')
        host=artifacts.verify_host_store(common.dx0_mac_host_artifact_parent()/self.artifact['host_manifest_sha256'],
            self.artifact['windows_build_input_identity'],expected_branch=self.windows_branch,expected_input_count=self.windows_input_count)
        if host['build_receipt']['producer_source']!=self.artifact['producer_source'] or host['custody']['artifact']['id']!=self.artifact['artifact_id']:
            raise d.AdapterBoundaryError('AP0 producer/custody differs')
        artifacts.verify_fixture_store(common.dx0_mac_fixture_parent()/d.AGAIN_BUNDLE_MANIFEST_SHA256)
    def _observation(self,reply,request):
        if reply['state'] in {'absent','unknown'}:return None
        v=reply['observation']
        if v.get('schema')!='linux-vst-bridge-'+self.product.lower()+'-reservation/v1' or v.get('binding')!=request:
            raise d.AdapterBoundaryError('AP0 observation binding differs')
        try:
            self.validate_runtime(v['runtime_observation'])
            if (v['runtime_observation']['declared_inputs_sha256']!=request['declared_runtime_inputs_sha256']
                    or v['execution_input_sha256']!=acceptance_execution_input_sha256(request,v['runtime_observation'])):
                raise d.AdapterBoundaryError('AP0 runtime/execution input differs')
            kind=ObservationKind(v['kind'])
            if kind is ObservationKind.SUCCESS:
                if v['classification']!=self.product+'_SAMPLES_VERIFIED' or v['cleanup']!='COMPLETE' or v['protected']!='UNCHANGED':
                    raise d.AdapterBoundaryError('AP0 disposition differs')
                self.check_summary(v['summary'])
        except Exception as error:
            return Observation(ObservationKind.INCONCLUSIVE,self.product+'_ADMISSION_FAILED',
                {'acceptance_eligible':False,'error':exception_detail(error),'available':checkpoint_projection(v['summary'])},
                v.get('cleanup','UNKNOWN'),v.get('protected','UNKNOWN'),reply.get('effects',{}))
        return Observation(kind,v['classification'],{'schema':self.product.lower()+'-classified-observation/v1',
            'execution_class':request['execution_class'],'acceptance_eligible':self.acceptance and kind is ObservationKind.SUCCESS,
            'binding':request,'document_sha256':sha256_bytes(canonical_json(v)),'document':v},v['cleanup'],v['protected'],reply.get('effects',{}))
    def admit(self,context,observation):
        delegation=self._validate_context(context.delegation,context.plan_descriptor)
        request=self._request(delegation,context.reservation_identity)
        payload=observation.payload
        if payload.get('schema')!=self.product.lower()+'-classified-observation/v1' or payload['binding']!=request:
            if not self.acceptance and observation.kind is not ObservationKind.SUCCESS:return payload
            raise d.AdapterBoundaryError('AP0 payload differs')
        restored=self._observation({'state':'observed','observation':payload['document'],'effects':observation.effects},request)
        if restored.record()!=observation.record() or (self.acceptance and payload['acceptance_eligible'] is not True):
            raise d.AdapterBoundaryError('AP0 observation cannot be admitted')
        return payload
    def render(self,context,result):
        observed=Observation.from_record(result['observation'])
        if self.admit(context,observed)!=result['admitted_result'] or not self.acceptance:
            raise d.AdapterBoundaryError('AP0 renderer requires fresh acceptance')
        sys.path.insert(0,str(self.repository/'tools/wf0-factory-census'))
        spec=importlib.util.spec_from_file_location('ap0_evidence',self.repository/'tools/wf0-factory-census/evidence.py')
        module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
        consumer=d._checked_text(self.ports.command.run(('git','rev-parse','HEAD'),cwd=self.repository),'AP0 consumer')
        module.render_ap0_packet(self.repository/EVIDENCE,result,consumer_source=consumer,validate=validate_summary)

def adapters(ports,repository=ROOT):
    path=repository/ARTIFACT
    if not path.exists():return {}
    artifact=json.loads(path.read_bytes());registry={}
    if set(artifact)!={'producer_source','windows_build_input_identity','producer_run_id','producer_run_attempt','artifact_id','host_manifest_sha256'}:
        raise d.AdapterBoundaryError('AP0 artifact binding roster differs')
    for cls in (ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE,ExecutionClass.ACCEPTANCE_CANDIDATE):
        plan=descriptor(cls,artifact);runtime=AP0Runtime(ports,plan,artifact,repository=repository)
        common=dict(descriptor=plan,preflight=runtime.preflight,reconcile=runtime.reconcile,invoke=runtime.invoke_diagnostic,admit=runtime.admit)
        registry[plan.plan_id]=(AcceptancePlanAdapter(**common,render_product_evidence=runtime.render) if runtime.acceptance else DiagnosticPlanAdapter(**common))
    return registry

"""Fresh PC0 acceptance over the shared classified execution/retention path."""
from __future__ import annotations
import importlib.util
import pathlib
import sys
from typing import Mapping

import pc0_proof_adapter as d
from pc0_contract import validate_acceptance_summary, acceptance_execution_input_sha256
from pc0_diagnostic_runtime import (declared_runtime_inputs, validate_runtime_observation,
                                    exception_detail, checkpoint_projection)
from classified_proof_backend import AcceptancePlanAdapter, Observation, ObservationKind, PlanDescriptor
from proof_execution_policy import ExecutionClass, canonical_json, sha256_bytes

PLAN_ID = 'pc0-pre-setup-processing-contract-acceptance-v1'
RUNTIME_REQUIREMENT = {**d.RUNTIME_REQUIREMENT,
    'declared_runtime_inputs_sha256':declared_runtime_inputs(),
    'metadata_policy':'exact-deployed-inputs-and-installed-selection; record-incidental-Steam-metadata'}
PLAN_DESCRIPTOR = PlanDescriptor.create(
    plan_id=PLAN_ID, execution_class=ExecutionClass.ACCEPTANCE_CANDIDATE,
    product_contract_identity=d.PRODUCT_CONTRACT_IDENTITY, product_contract_bytes=d._selection_bytes(),
    operation='pc0-fresh-pre-setup-acceptance', artifact_requirement=d.ARTIFACT_REQUIREMENT,
    fixture_requirement=d.FIXTURE_REQUIREMENT, runtime_requirement=RUNTIME_REQUIREMENT)
PLAN_CONTENT_SHA256 = PLAN_DESCRIPTOR.plan_content_sha256
WORKER_PATH = 'tools/pc0_acceptance_worker.py'
WORKER_SOURCE = (d.REPOSITORY_ROOT / WORKER_PATH).read_text()
SUPPORT_SOURCES = {**d.SUPPORT_SOURCES,
                  'pc0_contract':(d.REPOSITORY_ROOT / 'tools/pc0_contract.py').read_text()}
PROGRAM = WORKER_SOURCE.replace('_SUPPORT_SOURCES = {}', '_SUPPORT_SOURCES = '+repr(SUPPORT_SOURCES))
WORKER_SHA256 = sha256_bytes(PROGRAM.encode())
SCHEMA = 'linux-vst-bridge-pc0-reservation-acceptance/v1'
PAYLOAD_SCHEMA = 'linux-vst-bridge-pc0-classified-acceptance/v1'
DEPENDENCIES = tuple(sorted({
    'tools/proof-run.py', 'tools/proof_execution_policy.py', 'tools/classified_proof_backend.py',
    'tools/pc0_proof_adapter.py', 'tools/pc0_acceptance_adapter.py', WORKER_PATH,
    'tools/wf0-factory-census/evidence.py', 'tools/wf0-factory-census/common.py',
    'tools/wr0-proton-bootstrap/launch.py',
    *('tools/'+name+'.py' for name in SUPPORT_SOURCES)}))
FROZEN_DEPENDENCIES = (
    'tools/wf0-factory-census/common.py', 'tools/wf0-factory-census/artifacts.py',
    'tools/wf0-factory-census/environment.py', 'tools/wf0-factory-census/normalize.py',
    'tools/wf0-factory-census/supervise.py', 'tools/wf0-factory-census/run.py',
    'tools/wr0-proton-bootstrap/launch.py',
)
EVIDENCE_PATH = 'evidence/pc0-windows-vst3-pre-setup-processing-contract'


def candidate_identity(source):
    return sha256_bytes(canonical_json({'schema':'pc0-acceptance-candidate/v1',
        'source_commit':source, 'plan':PLAN_DESCRIPTOR.record()}))


class PC0AcceptanceRuntime(d.PC0DiagnosticRuntime):
    publication_effect = "acceptance_publications"
    worker_path, worker_source = WORKER_PATH, WORKER_SOURCE
    support_sources, worker_sha256, remote_program = SUPPORT_SOURCES, WORKER_SHA256, PROGRAM

    def __init__(self, *args, evidence_root=None, **kwargs):
        super().__init__(*args, **kwargs)
        self.evidence_root = evidence_root or self.repository / EVIDENCE_PATH

    def _validate_context(self, delegation_value, descriptor_value):
        value = dict(delegation_value)
        if (dict(descriptor_value) != PLAN_DESCRIPTOR.record()
                or value.get('execution_class') != 'ACCEPTANCE_CANDIDATE'
                or value.get('acceptance_eligible') is not True
                or value.get('batch_budget_maximum') != 1
                or value.get('plan_id') != PLAN_ID
                or value.get('plan_content_sha256') != PLAN_CONTENT_SHA256
                or value.get('product_contract_identity') != d.PRODUCT_CONTRACT_IDENTITY
                or value.get('product_contract_sha256') != d.SELECTION_SHA256
                or d.HEX40.fullmatch(str(value.get('source_commit'))) is None
                or value.get('execution_identity') != candidate_identity(value['source_commit'])):
            raise d.AdapterBoundaryError('PC0 acceptance candidate or plan differs')
        return value

    def _dependencies(self, commit):
        def records(source, paths):
            return [{'path':path, 'git_blob':d._checked_text(self.ports.command.run(
                ('git','rev-parse',source+':'+path),cwd=self.repository),'execution dependency')}
                for path in paths]
        return {
            'candidate':{'commit':commit, 'tree':d._checked_text(self.ports.command.run(
                ('git','rev-parse',commit+'^{tree}'),cwd=self.repository),'candidate tree'),
                'records':records(commit,DEPENDENCIES)},
            'retained_deck_source':{'commit':d.STOPPED_PC0_SOURCE,'tree':d.STOPPED_PC0_TREE,
                                   'records':records(d.STOPPED_PC0_SOURCE,FROZEN_DEPENDENCIES)}}

    def _request(self, delegation, reservation_identity):
        request = super()._request(delegation, reservation_identity)
        request['candidate_identity'] = request.pop('campaign_identity')
        request.update(acceptance_eligible=True, plan_content_sha256=PLAN_CONTENT_SHA256,
            declared_runtime_inputs_sha256=RUNTIME_REQUIREMENT['declared_runtime_inputs_sha256'],
            execution_dependencies=self._dependencies(delegation['source_commit']))
        return request

    def _local_preflight(self, delegation):
        super()._local_preflight(delegation)
        for record in self._dependencies(delegation['source_commit'])['candidate']['records']:
            raw = self.ports.filesystem.read_bytes(self.repository/record['path'],2*1024*1024)
            if d._git_blob_sha1(raw) != record['git_blob']:
                raise d.AdapterBoundaryError('candidate execution dependency differs: '+record['path'])

    def _observation(self, reply, request):
        try:
            return self._validated_observation(reply,request)
        except Exception as error:
            value=reply.get('observation')
            if (not isinstance(value,dict) or value.get('schema') != SCHEMA
                    or value.get('binding') != request):
                raise
            # A bound result can be retained but rejected. Keep the first Mac
            # admission error without turning a malformed record into evidence.
            return Observation(ObservationKind.INCONCLUSIVE,'PC0_ACCEPTANCE_ADMISSION_FAILED',
                {'schema':'pc0-acceptance-admission-error/v1','execution_class':'ACCEPTANCE_CANDIDATE',
                 'acceptance_eligible':False,'stage':'acceptance_admission',
                 'error':exception_detail(error),
                 'available':checkpoint_projection(value.get('summary',{}))},
                'UNKNOWN','UNKNOWN',reply.get('effects',{}))

    def _validated_observation(self, reply, request):
        if reply['state'] in {'absent','unknown'}:
            return None
        value = d._keys(reply.get('observation'), {
            'schema','binding','execution_input_sha256','kind','classification',
            'summary','cleanup','protected','runtime_observation'}, 'acceptance observation')
        if (value['schema'] != SCHEMA or value['binding'] != request
                or d.HEX64.fullmatch(str(value['execution_input_sha256'])) is None):
            raise d.AdapterBoundaryError('fresh acceptance observation binding differs')
        validate_runtime_observation(value['runtime_observation'])
        if value['runtime_observation']['declared_inputs_sha256'] != request['declared_runtime_inputs_sha256']:
            raise d.AdapterBoundaryError('acceptance runtime inputs differ')
        if value['execution_input_sha256'] != acceptance_execution_input_sha256(request,value['runtime_observation']):
            raise d.AdapterBoundaryError('acceptance execution input differs')
        kind = ObservationKind(value['kind'])
        if kind is ObservationKind.SUCCESS:
            if (value['classification'] != 'PC0_ACCEPTANCE_OBSERVED'
                    or value['cleanup'] != 'COMPLETE' or value['protected'] != 'UNCHANGED'
                    or value['summary'].get('run_id') != request['reservation_identity'][:32]):
                raise d.AdapterBoundaryError('acceptance shutdown/disposition differs')
            validate_acceptance_summary(value['summary'])
        elif not isinstance(value['summary'],dict) or len(canonical_json(value['summary'])) > 128*1024:
            raise d.AdapterBoundaryError('acceptance failure record exceeds bound')
        return Observation(kind, value['classification'], {
            'schema':PAYLOAD_SCHEMA, 'execution_class':'ACCEPTANCE_CANDIDATE',
            'acceptance_eligible':kind is ObservationKind.SUCCESS, 'binding':dict(request),
            'document_sha256':sha256_bytes(canonical_json(value)), 'document':value},
            value['cleanup'], value['protected'], reply.get('effects',{}))

    def admit_acceptance(self, context, observation):
        delegation = self._validate_context(context.delegation, context.plan_descriptor)
        request = self._request(delegation,context.reservation_identity)
        payload = d._keys(observation.payload, {'schema','execution_class','acceptance_eligible',
            'binding','document_sha256','document'}, 'acceptance payload')
        if (observation.kind is not ObservationKind.SUCCESS
                or payload['schema'] != PAYLOAD_SCHEMA
                or payload['execution_class'] != 'ACCEPTANCE_CANDIDATE'
                or payload['acceptance_eligible'] is not True or payload['binding'] != request
                or payload['document_sha256'] != sha256_bytes(canonical_json(payload['document']))):
            raise d.AdapterBoundaryError('diagnostic, checkpoint or failure cannot satisfy acceptance')
        validated = self._observation({'state':'observed','observation':payload['document'],
                                       'effects':observation.effects},request)
        if validated.record() != observation.record():
            raise d.AdapterBoundaryError('acceptance observation projection differs')
        return payload

    def render(self, context, result):
        # This is a local consumer of the already admitted immutable acceptance
        # result. No SSH, fixture, process, or workload method is reachable here.
        observation = Observation.from_record(result['observation'])
        admitted = self.admit_acceptance(context,observation)
        if admitted != result['admitted_result'] or result['acceptance_eligible'] is not True:
            raise d.AdapterBoundaryError('renderer acceptance result differs')
        path = self.repository/'tools/wf0-factory-census/evidence.py'
        sys.path.insert(0,str(path.parent))
        spec = importlib.util.spec_from_file_location('pc0_acceptance_evidence',path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        consumer_source=d._checked_text(self.ports.command.run(
            ('git','rev-parse','HEAD'),cwd=self.repository),'renderer source')
        module.render_pc0_acceptance_packet(self.evidence_root, result,
            consumer_source=consumer_source,
            validate=validate_acceptance_summary)


def create_pc0_acceptance_adapter(ports=None, *, repository=d.REPOSITORY_ROOT,
                                  proof_root=None, evidence_root=None):
    runtime = PC0AcceptanceRuntime(ports or d.production_ports(),repository=repository,
        proof_root=proof_root,evidence_root=evidence_root)
    return AcceptancePlanAdapter(descriptor=PLAN_DESCRIPTOR, preflight=runtime.preflight,
        reconcile=runtime.reconcile,invoke=runtime.invoke_diagnostic,
        admit=runtime.admit_acceptance,render_product_evidence=runtime.render)

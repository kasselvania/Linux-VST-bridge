"""Complete classified acceptance chain with only platform/process work replaced."""
import ast
import contextlib
import copy
import importlib.util
import io
import json
import pathlib
import subprocess
import sys
import unittest
from unittest.mock import patch

TOOLS=pathlib.Path(__file__).resolve().parent
sys.path.insert(0,str(TOOLS))
import pc0_acceptance_adapter as a
import pc0_proof_adapter as d
import pc0_contract as contract
from classified_proof_backend import ClassifiedProofBackend, Observation, ReservationContext
from proof_execution_policy import canonical_json
from test_pc0_proof_adapter import PC0AdapterTests, FakeCommandPort, SOURCE_A, STOPPED_PC0_SOURCE
from test_proof_cli import cli


class AcceptanceCommand(FakeCommandPort):
    def run(self, argv, **kwargs):
        target=argv[-1]
        if tuple(argv[:2]) == ('git','rev-parse'):
            if target == SOURCE_A+'^{tree}':
                return d.CommandReply(0,b'a'*40+b'\n',b'')
            if ':' in target:
                source,path=target.split(':',1)
                if path in a.DEPENDENCIES or path in a.FROZEN_DEPENDENCIES:
                    if source == SOURCE_A:
                        raw=(TOOLS.parent/path).read_bytes()
                    else:
                        raw=subprocess.check_output(['git','show',target],cwd=TOOLS.parent)
                    return d.CommandReply(0,(d._git_blob_sha1(raw)+'\n').encode(),b'')
        return super().run(argv,**kwargs)


class AcceptanceTests(unittest.TestCase):
    setUpClass=classmethod(PC0AdapterTests.setUpClass.__func__)
    tearDownClass=classmethod(PC0AdapterTests.tearDownClass.__func__)
    mock=PC0AdapterTests.mock
    pair=PC0AdapterTests.pair
    snapshot=PC0AdapterTests.snapshot
    retirement=PC0AdapterTests.retirement
    scan=PC0AdapterTests.scan
    complete_synthetic_scan=PC0AdapterTests.complete_synthetic_scan

    def setUp(self):
        PC0AdapterTests.setUp(self)
        # A nonempty realistic scanner stream through the actual normalizer.
        self.expected,self.synthetic_scan=self.complete_synthetic_scan()
        self.source.update(schema='local-test-source',record_count=0,records=[],tree='a'*40)
        self.command=AcceptanceCommand()
        ssh=d.StrictSSHPort(self.local_worker);ssh._destination='local-effect-free-port'
        self.runtime=a.PC0AcceptanceRuntime(d.AdapterPorts(self.command,d.OSFileSystemPort(),ssh),
            repository=TOOLS.parent,proof_root=self.mac_proof,evidence_root=self.base/'evidence')
        from classified_proof_backend import AcceptancePlanAdapter
        self.adapter=AcceptancePlanAdapter(descriptor=a.PLAN_DESCRIPTOR,preflight=self.runtime.preflight,
            reconcile=self.runtime.reconcile,invoke=self.runtime.invoke_diagnostic,
            admit=self.runtime.admit_acceptance,render_product_evidence=self.runtime.render)
        self.backend=ClassifiedProofBackend(self.base/'state',{a.PLAN_ID:self.adapter})
        # The worker/helper map now includes the exact shared contract module.
        self.stack.enter_context(patch.object(contract,'_pc0_source_sha256',
            d.sha256_bytes(a.SUPPORT_SOURCES['pc0_contract'].encode()),create=True))
        original=self.synthetic_scan
        # Construct the fixed synthetic call returns independently of live data.
        common_source=subprocess.check_output(['git','show',STOPPED_PC0_SOURCE+':tools/wf0-factory-census/common.py'],cwd=TOOLS.parent).decode()
        function=next(n for n in ast.parse(common_source).body if isinstance(n,ast.FunctionDef) and n.name=='pc0_validate_call_facts')
        inherited=ast.literal_eval(next(n.value for n in function.body if isinstance(n,ast.Assign) and n.targets[0].id=='inherited'))
        inherited_indexes=dict(zip([*range(14),*range(25,33)],inherited))
        def scan(*args,**kwargs):
            result=original(*args,**kwargs)
            calls=[r for r in result['records'] if r['event'] in {'call_started','call_completed'}]
            for index in range(33):
                start,end=calls[index*2:index*2+2]
                if index in inherited_indexes:
                    interface,start_fields,end_fields=inherited_indexes[index]
                    start.update(interface=interface,**start_fields)
                    end.pop('result_u32_hex',None)
                    end.update(interface=interface,**end_fields)
                else:
                    end['return_kind']='int32' if start['operation']=='get_bus_count' else 'tresult'
                    if start['operation']=='get_bus_count': end.pop('result_u32_hex',None)
                start.setdefault('ordinal',None);start.setdefault('tier',None)
                end['ordinal']=start['ordinal'];end['tier']=start['tier']
            result.update(run_id=self.current_reservation[:32],last_lifecycle='scanner_completed',
                inherited_shutdown={'operations':{op:{'disposition':'completed','source':'scanner_call_ledger'}
                    for op in contract.SHUTDOWN_OPERATIONS},'clean_in_process_shutdown':True,
                    'physical_containment_only':False})
            return result
        self.supervise.side_effect=scan
        fields={'status':'active_acceptance_candidate','authority_phase':'implementation',
            'change_class':'PRODUCT_CONTRACT_CHANGE','product_implementation_authorized':'true',
            'live_execution_authorized':'true','permitted_execution_class':'ACCEPTANCE_CANDIDATE',
            'classified_backend_core_ready':'true','classified_backend_ready':'true',
            'authorized_source_commit':SOURCE_A,'authorized_plan_id':a.PLAN_ID,
            'authorized_product_contract_identity':d.PRODUCT_CONTRACT_IDENTITY,
            'authorized_product_contract_sha256':d.SELECTION_SHA256,
            'authorized_plan_content_sha256':a.PLAN_CONTENT_SHA256,
            'acceptance_candidate_identity':a.candidate_identity(SOURCE_A),'acceptance_batch_budget':'1'}
        self.authority=self.base/'PC0_A1.md'
        self.authority.write_text('## Authority\n```yaml\n'+'\n'.join(f'{k}: {v}' for k,v in fields.items())+'\n```\n')
        from proof_execution_policy import load_authority,authorize_live_request,LiveRequest,ExecutionClass
        self.auth=load_authority(self.authority)
        self.delegation=authorize_live_request(self.auth,LiveRequest(ExecutionClass.ACCEPTANCE_CANDIDATE,
            SOURCE_A,a.PLAN_ID,a.candidate_identity(SOURCE_A)))
        self.current_reservation=self.backend._reservation_identity(self.delegation)
        self.args=['accept','--authority',str(self.authority),'--source',SOURCE_A,
                   '--plan',a.PLAN_ID,'--candidate',a.candidate_identity(SOURCE_A)]
        self.stack.enter_context(patch.object(cli,'_production_adapters',return_value=({a.PLAN_ID:self.adapter},type('Command',(),{'last_failure':None})())))
        self.stack.enter_context(patch.object(cli,'STATE_ROOT',self.base/'state'))

    def execute_cli(self,extra=()):
        output=io.StringIO();error=io.StringIO()
        with contextlib.redirect_stdout(output),contextlib.redirect_stderr(error):
            rc=cli.main([*self.args,*extra])
        self.assertEqual(rc,0,error.getvalue())
        return json.loads(output.getvalue())

    def transaction(self):
        path=self.base/'state/acceptance'/a.candidate_identity(SOURCE_A)/'transactions'/self.current_reservation/'transaction.json'
        return json.loads(path.read_bytes())

    def test_real_command_worker_normalizer_admission_renderer(self):
        self.assertEqual(self.execute_cli(['--preflight-only'])['state'],'ready')
        self.assertEqual(self.launches,0)
        self.assertFalse((self.base/'state').exists())
        receipt=self.execute_cli()
        self.assertTrue(receipt['renderer_completed'],self.transaction())
        self.assertEqual(receipt['budget_consumed'],1)
        self.assertEqual(self.launches,1)
        self.assertEqual(len(list((self.base/'evidence').iterdir())),5)
        packet=json.loads((self.base/'evidence/TRANSACTION.json').read_bytes())
        self.assertEqual(packet['result']['admitted_result']['document']['summary']['processing_contract'],self.expected)
        self.assertTrue(packet['result']['acceptance_eligible'])
        self.assertEqual(self.execute_cli()['budget_consumed'],1)
        self.assertEqual(self.launches,1)

    def context(self):
        t=self.transaction()
        return ReservationContext(t['delegation'],t['plan_descriptor'],t['reservation_identity'],t)

    def test_wrong_class_and_malformed_observed_facts_cannot_satisfy_acceptance(self):
        self.execute_cli()
        original=self.transaction()['observation']
        mutations = {
            'diagnostic_class':lambda v:v['payload'].update(execution_class='DIAGNOSTIC_NON_AUTHORITATIVE'),
            'diagnostic_eligibility':lambda v:v['payload'].update(acceptance_eligible=False),
            'checkpoint':lambda v:v['payload']['document'].update(summary={'troubleshooting':{}}),
            'failed':lambda v:v.update(kind='FAILED'),
            'wrong_source':lambda v:v['payload']['document']['binding'].update(adapter_source_commit='e'*40),
            'wrong_artifact':lambda v:v['payload']['document']['binding'].update(artifact_id=1),
            'wrong_execution_input':lambda v:v['payload']['document'].update(execution_input_sha256='0'*64),
            'wrong_plan':lambda v:v['payload']['document']['binding'].update(plan_content_sha256='0'*64),
            'runtime':lambda v:v['payload']['document']['runtime_observation'].update(declared_inputs_sha256='0'*64),
            'bus_count':lambda v:v['payload']['document']['summary']['processing_contract']['counts'][0].update(count=2),
            'bus_info':lambda v:v['payload']['document']['summary']['processing_contract']['buses'][0].update(channel_count=3),
            'arrangement':lambda v:v['payload']['document']['summary']['processing_contract']['buses'][0]['speaker_arrangement'].update(bits_u64_hex='0000000000000001'),
            'sample_format':lambda v:v['payload']['document']['summary']['processing_contract']['sample_sizes'][0].update(supported=False),
            'extra_call':lambda v:v['payload']['document']['summary']['verification']['call_facts']['ledger'].append({}),
            'lifecycle_order':lambda v:v['payload']['document']['summary']['verification']['lifecycle'].reverse(),
            'missing_return':lambda v:v['payload']['document']['summary']['verification']['call_facts']['ledger'].pop(),
            'shutdown':lambda v:v['payload']['document']['summary']['shutdown'].update(clean_in_process_shutdown=False),
            'containment':lambda v:v['payload']['document']['summary']['verification']['containment'].update(owned_descendants_zero=False),
            'stage':lambda v:v['payload']['document']['summary']['verification'].update(stage_absent=False),
            'protected':lambda v:v['payload']['document']['summary']['verification'].update(protected_after_sha256='0'*64),
        }
        for name,mutate in mutations.items():
            with self.subTest(name=name):
                value=copy.deepcopy(original);mutate(value)
                value['payload']['document_sha256']=d.sha256_bytes(canonical_json(value['payload']['document']))
                with self.assertRaises(Exception):
                    self.runtime.admit_acceptance(self.context(),Observation.from_record(value))
        self.assertEqual(self.launches,1)

    def test_preflight_fixture_runtime_and_source_rejection_spends_nothing(self):
        for stage in ('source','fixture','runtime','plan'):
            with self.subTest(stage=stage),contextlib.ExitStack() as stack:
                if stage=='source':
                    stack.enter_context(patch.object(self.command,'head_source','e'*40))
                elif stage=='fixture':
                    stack.enter_context(patch.dict(self.host['custody']['artifact'],id=1))
                elif stage=='runtime':
                    stack.enter_context(patch.dict(self.runtime_observation,declared_inputs_sha256='0'*64))
                else:
                    stack.enter_context(patch.object(a,'PLAN_CONTENT_SHA256','0'*64))
                output=io.StringIO();error=io.StringIO()
                with contextlib.redirect_stdout(output),contextlib.redirect_stderr(error):
                    self.assertEqual(cli.main([*self.args,'--preflight-only']),2)
                self.assertTrue(error.getvalue())
                self.assertEqual(self.launches,0)
                self.assertFalse((self.base/'state').exists())

    def test_lost_acknowledgement_and_reinvocation_never_duplicate_workload(self):
        self.local_worker.lose_ack=True
        first=self.execute_cli()
        self.assertTrue(first['renderer_completed'])
        self.assertEqual(self.launches,1)
        self.execute_cli()
        self.assertEqual(self.launches,1)
        self.assertEqual(self.local_worker.actions.count('execute'),1)

    def test_reporting_recovery_uses_retained_acceptance_without_remote_calls(self):
        render=self.adapter.render_product_evidence
        from dataclasses import replace
        broken=replace(self.adapter,render_product_evidence=lambda *_:(_ for _ in ()).throw(OSError('local render failure')))
        with patch.object(cli,'_production_adapters',return_value=({a.PLAN_ID:broken},type('Command',(),{'last_failure':None})())):
            first=self.execute_cli()
        self.assertFalse(first['renderer_completed'])
        t=self.transaction()
        result_path=self.base/'state/acceptance'/a.candidate_identity(SOURCE_A)/'transactions'/self.current_reservation/'classified-result.json'
        retained=result_path.read_bytes()
        before=list(self.local_worker.actions)
        with patch.object(self.runtime,'preflight',side_effect=AssertionError('no remote preflight')), \
             patch.object(self.local_worker,'run',side_effect=AssertionError('no SSH')):
            second=self.execute_cli(['--render-only'])
        self.assertTrue(second['renderer_completed'])
        self.assertEqual(self.launches,1)
        self.assertEqual(self.local_worker.actions,before)
        self.assertEqual(result_path.read_bytes(),retained)
        self.assertEqual(self.transaction()['failure_sha256'],t['failure_sha256'])
        self.assertEqual(second['budget_consumed'],1)

    def test_failed_acceptance_retains_useful_error_and_never_renders_or_retries(self):
        self.normalizer.side_effect=ValueError('synthetic normalization failure')
        receipt=self.execute_cli()
        self.assertFalse(receipt['renderer_completed'])
        self.assertEqual(receipt['budget_remaining'],0)
        self.assertFalse((self.base/'evidence').exists())
        document=self.transaction()['observation']['payload']['document']
        self.assertIn('synthetic normalization failure',str(document['summary']))
        self.assertEqual(document['cleanup'],'COMPLETE')
        self.assertFalse(self.transaction()['observation']['payload']['acceptance_eligible'])
        self.execute_cli()
        self.assertEqual(self.launches,1)

    def test_product_validators_are_exact_frozen_extractions(self):
        frozen=subprocess.check_output(['git','show',STOPPED_PC0_SOURCE+':tools/wf0-factory-census/common.py'],cwd=TOOLS.parent).decode()
        original={n.name:ast.dump(n) for n in ast.parse(frozen).body if isinstance(n,ast.FunctionDef)}
        current=ast.parse((TOOLS/'pc0_contract.py').read_text())
        for node in current.body:
            if isinstance(node,ast.FunctionDef) and node.name in {'pc0_expected_contract','pc0_validate_contract','pc0_validate_call_facts'}:
                self.assertEqual(ast.dump(node),original[node.name])

def load_tests(loader, tests, pattern):
    return loader.loadTestsFromTestCase(AcceptanceTests)


if __name__=='__main__':unittest.main()

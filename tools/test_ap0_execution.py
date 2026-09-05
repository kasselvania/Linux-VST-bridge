"""Actual AP0 command/worker/checker/retention flow; platform work substituted."""
import ast,copy,importlib.util,io,json,pathlib,sys,types,unittest
from unittest.mock import patch
TOOLS=pathlib.Path(__file__).parent.resolve();sys.path.insert(0,str(TOOLS))
import ap0_adapter as a
import ap0_contract as contract
import pc0_proof_adapter as d
from test_ap0_contract import blocks
from test_pc0_acceptance import AcceptanceTests
from test_pc0_proof_adapter import PC0AdapterTests,SOURCE_A
from test_proof_cli import cli
from classified_proof_backend import ClassifiedProofBackend,AcceptancePlanAdapter,Observation
from proof_execution_policy import *

class AP0ExecutionTests(AcceptanceTests):
    def setUp(self):
        super().setUp()
        artifact={k:self.runtime._request(self.delegation,None)[k] for k in (
            'producer_source','windows_build_input_identity','producer_run_id',
            'producer_run_attempt','artifact_id','host_manifest_sha256')}
        plan=a.descriptor(ExecutionClass.ACCEPTANCE_CANDIDATE,artifact)
        runtime=a.AP0Runtime(self.runtime.ports,plan,artifact,repository=TOOLS.parent,proof_root=self.mac_proof)
        self.runtime=runtime;self.plan=plan
        self.stack.enter_context(patch.object(runtime,'_dependencies',return_value={'candidate':{'commit':SOURCE_A,'tree':'a'*40,'records':[]},'retained_deck_source':{'commit':d.STOPPED_PC0_SOURCE,'tree':d.STOPPED_PC0_TREE,'records':[]}}))
        # The existing PC0 suites exercise actual local admission with fake file/
        # command ports. Here only effectful local custody/source work is stubbed.
        self.local=self.stack.enter_context(patch.object(runtime,'_local_preflight'))
        for name in ('ap0_artifacts','ap0_worker_support'):
            module=types.ModuleType(name);module.__file__='<'+name+'>'
            module._pc0_source_sha256=d.sha256_bytes(a.SUPPORT[name].encode())
            self.stack.enter_context(patch.dict(sys.modules,{name:module}))
            exec(compile(a.SUPPORT[name],module.__file__,'exec'),module.__dict__)
        self.stack.enter_context(patch.object(sys.modules['ap0_worker_support'],'verify_host',return_value=self.host))
        self.stack.enter_context(patch.object(contract,'_pc0_source_sha256',d.sha256_bytes(a.SUPPORT['ap0_contract'].encode()),create=True))
        self.original=self.supervise.side_effect
        def scan(*args,**kwargs):
            self.assertEqual(kwargs.pop("mode"),contract.MODE)
            kwargs["mode"]=self.common.PC0_MODE
            kwargs.pop("profile",None)
            r=self.original(*args,**kwargs)
            before=next(i for i,v in enumerate(r['records']) if v.get('operation')=='release_audio_processor' and v['event']=='call_started')
            extra=[]
            def add(state,**fields):extra.append(dict(event='lifecycle',state=state,**fields))
            for op in contract.CALLS:
                if op=='set_processing_true':add('ap0_processing_thread_started',distinct_from_owner=True)
                add('ap0_call_started',operation=op,owner_thread=op not in ('set_processing_true','set_processing_false'))
                add('ap0_call_completed',operation=op,result=0)
                if op=='set_processing_true':
                    for b in range(3):
                        add('ap0_process_started',block=b);add('ap0_process_completed',block=b,result=0)
                if op=='set_processing_false':add('ap0_thread_joined',joined=True,processing_stopped=True,worker_exception=False)
            for block in blocks():add('ap0_samples',**block)
            r['records'][before:before]=extra
            mapping={v['sequence']:i+1 for i,v in enumerate(r['records']) if 'sequence' in v}
            for i,v in enumerate(r['records']):
                for key in ('attempt_sequence','enclosing_attempt_sequence'):
                    if v.get(key) in mapping:v[key]=mapping[v[key]]
                v['sequence']=i+1
            return r
        self.supervise.side_effect=scan
        self.adapter=AcceptancePlanAdapter(descriptor=plan,preflight=runtime.preflight,reconcile=runtime.reconcile,invoke=runtime.invoke_diagnostic,admit=runtime.admit,render_product_evidence=lambda c,r:self.render(c,r))
        self.backend=ClassifiedProofBackend(self.base/'state',{plan.plan_id:self.adapter})
        fields=dict(status='active_acceptance_candidate',authority_phase='implementation',change_class='PRODUCT_CONTRACT_CHANGE',product_implementation_authorized='true',live_execution_authorized='true',permitted_execution_class='ACCEPTANCE_CANDIDATE',classified_backend_core_ready='true',classified_backend_ready='true',authorized_source_commit=SOURCE_A,authorized_plan_id=plan.plan_id,authorized_product_contract_identity=plan.product_contract_identity,authorized_product_contract_sha256=plan.product_contract_sha256,authorized_plan_content_sha256=plan.plan_content_sha256,acceptance_candidate_identity=a.candidate_identity(SOURCE_A,plan),acceptance_batch_budget='1')
        self.authority.write_text('## Authority\n```yaml\n'+'\n'.join(f'{k}: {v}' for k,v in fields.items())+'\n```\n')
        self.auth=load_authority(self.authority)
        self.delegation=authorize_live_request(self.auth,LiveRequest(ExecutionClass.ACCEPTANCE_CANDIDATE,SOURCE_A,plan.plan_id,fields['acceptance_candidate_identity']))
        self.current_reservation=self.backend._reservation_identity(self.delegation)
        self.args=['accept','--authority',str(self.authority),'--source',SOURCE_A,'--plan',plan.plan_id,'--candidate',fields['acceptance_candidate_identity']]
        self.stack.enter_context(patch.object(cli,'_production_adapters',return_value=({plan.plan_id:self.adapter},types.SimpleNamespace(last_failure=None))))
        self.rendered=[]
    def render(self,context,result):
        self.runtime.admit(context,Observation.from_record(result['observation']))
        spec=importlib.util.spec_from_file_location('ap0_test_evidence',TOOLS/'wf0-factory-census/evidence.py')
        m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
        m.render_ap0_packet(self.base/'ap0-evidence',result,consumer_source=SOURCE_A,validate=contract.validate_summary)
        self.rendered.append(result)
    def transaction(self):
        path=self.base/'state/acceptance'/self.delegation['execution_identity']/'transactions'/self.current_reservation/'transaction.json'
        return json.loads(path.read_bytes())
    def test_ap0_command_worker_retains_and_renders(self):
        self.assertEqual(self.execute_cli(['--preflight-only'])['state'],'ready')
        self.assertEqual(self.launches,0)
        receipt=self.execute_cli()
        self.assertTrue(receipt['renderer_completed'],self.transaction().get('observation',{}).get('payload',{}).get('document',{}).get('summary',{}))
        s=self.rendered[0]['admitted_result']['document']['summary']
        self.assertEqual(s['comparison']['samples_compared'],96)
        self.assertEqual(s['comparison']['maximum_absolute_error'],0)
        self.assertEqual(self.launches,1)
        self.execute_cli();self.assertEqual(self.launches,1)
    def test_ap0_bad_sample_retains_original_and_cleanup(self):
        original=self.supervise.side_effect
        def bad(*args,**kwargs):
            r=original(*args,**kwargs)
            next(v for v in r['records'] if v.get('state')=='ap0_samples')['output_bits'][0][1]=0x7fc12345
            return r
        self.supervise.side_effect=bad
        self.execute_cli();self.assertFalse(self.rendered)
        self.assertEqual(self.retire.call_count,1)
        root=self.remote_proof/'acceptance/ACCEPTANCE_CANDIDATE'/self.delegation['execution_identity']/self.current_reservation
        v=json.loads((root/'observation.json').read_bytes())
        self.assertEqual(v['kind'],'INCONCLUSIVE')
        self.assertEqual(v['cleanup'],'COMPLETE')
        self.assertIn('nonfinite',json.dumps(v['summary']))
        self.execute_cli();self.assertEqual(self.launches,1)
    def test_ap0_callback_stream_and_pairing(self):
        def started():
            stream=sys.modules['ap0_worker_support'].StreamState()
            stream.accept(dict(event='lifecycle',sequence=1,state='ap0_call_started',operation='set_active_true',owner_thread=True))
            return stream
        base=dict(event='host_callback',sequence=2,operation='createInstance',thread_role='scanner_main_thread',origin='component',enclosing_attempt_sequence=1,enclosing_operation='set_active_true',result_u32_hex='00000000',output_null=False)
        for change in ({'enclosing_attempt_sequence':9},{'thread_role':'processing_thread'},{'output_null':True}):
            stream=started()
            with self.assertRaises(RuntimeError):stream.accept({**base,**change})
            self.assertTrue(stream.records[-1]['rejected'])
        stream=started()
        callbacks=[dict(operation='addRef',reference_count=3),dict(operation='queryInterface'),dict(operation='createInstance'),dict(operation='release',reference_count=2)]
        for seq,change in enumerate(callbacks,2):stream.accept({**base,**change,'sequence':seq})
        stream.accept(dict(event='lifecycle',sequence=6,state='ap0_call_completed',operation='set_active_true',result=0))
        self.assertIsNone(stream.ap0_call)
        self.assertIsNone(stream.in_flight_at)
    def test_ap0_unimplemented_notification_retained(self):
        original=self.supervise.side_effect
        def scan(*args,**kwargs):
            r=original(*args,**kwargs)
            for v in r['records']:
                if v.get('state')=='ap0_call_completed' and v['operation'].startswith('set_processing_'):
                    v['result']=-2147467263
            return r
        self.supervise.side_effect=scan
        self.execute_cli()
        self.assertEqual(len(self.rendered),1)
        s=self.rendered[0]['admitted_result']['document']['summary']
        self.assertEqual([r['result'] for r in s['lifecycle'] if r.get('state')=='ap0_call_completed' and r['operation'].startswith('set_processing_')],[-2147467263]*2)
        for operation in contract.CALLS:
            self.assertEqual(contract.lifecycle_result_ok(operation,-2147467263),operation.startswith('set_processing_'))
            self.assertFalse(contract.lifecycle_result_ok(operation,-1))
            self.assertFalse(contract.lifecycle_result_ok(operation,False))
        joined=next(r for r in s['lifecycle'] if r['state']=='ap0_thread_joined')
        joined['joined']=False
        with self.assertRaisesRegex(ValueError,'not stopped/joined'):contract.validate_summary(s)

    def test_ap0_lost_ack_no_duplicate(self):
        self.local_worker.lose_ack=True
        self.execute_cli();self.execute_cli()
        self.assertEqual(self.launches,1)

def load_tests(loader,tests,pattern):
    return unittest.TestSuite(AP0ExecutionTests(name) for name in dir(AP0ExecutionTests) if name.startswith('test_ap0_'))

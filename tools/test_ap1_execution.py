"""AP1 actual classified worker/retention with effectful supervision substituted."""
import copy,importlib.util,json,pathlib,sys,types,unittest
from unittest.mock import patch
TOOLS=pathlib.Path(__file__).parent.resolve();sys.path.insert(0,str(TOOLS))
import ap1_adapter as a,ap1_contract as contract,pc0_proof_adapter as d
from test_ap0_execution import AP0ExecutionTests
from test_pc0_proof_adapter import SOURCE_A
from test_proof_cli import cli
from classified_proof_backend import ClassifiedProofBackend,AcceptancePlanAdapter,Observation
from proof_execution_policy import *
def caller():
    ready=dict(event='ap1_client_ready',seed=789,mapping_count=1,connection_count=1,mapping_witness=True,seed_chosen_after_ready=True)
    records=[ready]
    import struct
    for b,n in enumerate(contract.LENGTHS):
        ins=[contract.input_words(789,b,ch) for ch in range(2)];outs=[]
        for ch in range(2):
            out=[contract.GUARD]+[contract.POISON]*256+[contract.GUARD]
            for i in range(1,n+1):out[i]=struct.unpack('<I',struct.pack('<f',contract.sample(ins[ch][i])*contract.GAINS[b]))[0]
            outs.append(out)
        records.append(dict(event='ap1_client_block',sequence=b+1,frames=n,gain=contract.GAINS[b],silent=b==5,input_bits=ins,output_bits=outs,maximum_absolute_error=0.))
    records.append(dict(event='ap1_client_closed',blocks=8,samples_compared=1344,maximum_absolute_error=0.,closed_received=True,mapping_unmapped=True,replays=0))
    return dict(records=records,raw_exit=0,cleanup=dict(owned_descendants_zero=True,process_group_empty=True))
class AP1ExecutionTests(AP0ExecutionTests):
    def setUp(self):
        super().setUp();old=self.runtime
        client=dict(source_commit=SOURCE_A,input_sha256='1'*64,binary_sha256='2'*64,manifest_sha256='3'*64)
        plan=a.descriptor(ExecutionClass.ACCEPTANCE_CANDIDATE,old.artifact,client)
        r=a.AP1Runtime(old.ports,plan,old.artifact,client,repository=TOOLS.parent,proof_root=self.mac_proof);self.runtime=r;self.plan=plan
        self.stack.enter_context(patch.object(r,'_dependencies',old._dependencies));self.stack.enter_context(patch.object(r,'_local_preflight'))
        for name in ('ap1_client_artifact','ap1_contract','ap1_worker_support'):
            module=types.ModuleType(name);module.__file__='<'+name+'>';module._pc0_source_sha256=d.sha256_bytes(a.SUPPORT[name].encode())
            self.stack.enter_context(patch.dict(sys.modules,{name:module}));exec(compile(a.SUPPORT[name],module.__file__,'exec'),module.__dict__)
        self.profile=sys.modules['ap1_worker_support'];self.actual_profile_supervise=self.profile.supervise
        self.stack.enter_context(patch.object(self.profile,'verify_host',return_value=self.host));self.stack.enter_context(patch.object(self.profile,'bind_client'))
        original=self.supervise.side_effect
        def scan(*args,**kwargs):
            kwargs['mode']='ap0-offline-again-processing';r=original(*args,**kwargs)
            records=r['records'];records[:]=[v for v in records if v.get('state') not in {'ap0_samples','ap0_process_started','ap0_process_completed'}]
            index=next(i for i,v in enumerate(records) if v.get('state')=='ap0_call_completed' and v.get('operation')=='set_processing_true')+1
            extra=[]
            for b in range(8):
                extra.extend([dict(event='lifecycle',state='ap0_process_started',block=b),dict(event='lifecycle',state='ap0_process_completed',block=b,result=0),dict(event='lifecycle',state='ap1_private_buffers_valid',block=b)])
            records[index:index]=extra
            records.insert(0,dict(event='lifecycle',state='ap1_mapping_ready',mapping_count=1,connection_count=1,mapping_witness=True))
            records.append(dict(event='lifecycle',state='ap1_endpoint_closed',mapping_unmapped=True,instance_count=1))
            mapping={v['sequence']:i+1 for i,v in enumerate(records) if 'sequence' in v}
            for i,v in enumerate(records):
                for k in ('attempt_sequence','enclosing_attempt_sequence'):
                    if v.get(k) in mapping:v[k]=mapping[v[k]]
                v['sequence']=i+1
            r['caller']=caller();return r
        self.ap1_scan=self.stack.enter_context(patch.object(self.profile,'supervise',side_effect=scan))
        self.adapter=AcceptancePlanAdapter(descriptor=plan,preflight=r.preflight,reconcile=r.reconcile,invoke=r.invoke_diagnostic,admit=r.admit,render_product_evidence=lambda c,v:self.render(c,v))
        self.backend=ClassifiedProofBackend(self.base/'state',{plan.plan_id:self.adapter})
        fields=dict(self.auth.fields);fields.update(authorized_plan_id=plan.plan_id,authorized_product_contract_identity=plan.product_contract_identity,authorized_product_contract_sha256=plan.product_contract_sha256,authorized_plan_content_sha256=plan.plan_content_sha256,acceptance_candidate_identity=a.candidate_identity(SOURCE_A,plan))
        self.authority.write_text('## Authority\n```yaml\n'+'\n'.join(f'{k}: {v}' for k,v in fields.items())+'\n```\n');self.auth=load_authority(self.authority)
        self.delegation=authorize_live_request(self.auth,LiveRequest(ExecutionClass.ACCEPTANCE_CANDIDATE,SOURCE_A,plan.plan_id,fields['acceptance_candidate_identity']))
        self.current_reservation=self.backend._reservation_identity(self.delegation)
        self.args=['accept','--authority',str(self.authority),'--source',SOURCE_A,'--plan',plan.plan_id,'--candidate',fields['acceptance_candidate_identity']]
        self.stack.enter_context(patch.object(cli,'_production_adapters',return_value=({plan.plan_id:self.adapter},types.SimpleNamespace(last_failure=None))))
    def render(self,context,result):
        self.runtime.admit(context,Observation.from_record(result['observation']))
        spec=importlib.util.spec_from_file_location('ap1_test_evidence',TOOLS/'wf0-factory-census/evidence.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
        m.render_ap1_packet(self.base/'ap1-evidence',result,consumer_source=SOURCE_A,validate=contract.validate_summary);self.rendered.append(result)
    def test_ap1_companion_cleanup_and_original_error_are_composed(self):
        from unittest.mock import MagicMock
        from types import SimpleNamespace
        environment=SimpleNamespace(session=self.base/'native-session');environment.session.mkdir()
        child=MagicMock(pid=321);child.poll.return_value=0
        def spawn(argv,**kwargs):
            self.assertEqual(argv[1:3],['--session-dir','.']);self.assertEqual(kwargs['cwd'],environment.session)
            (environment.session/'ap1.control').write_bytes(b'fixture')
            for record in caller()['records']:kwargs['stdout'].write((json.dumps(record)+'\n').encode())
            return child
        def windows(*args,**kwargs):
            kwargs['checkpoint']('supervisor_containment',{'records':[{'event':'lifecycle','state':'ap1_transport_error','detail':'original failure'}],
                'cleanup':{'owned_descendants_zero':True,'process_group_empty':True}})
            raise ValueError('original supervisor failure')
        history=[]
        with patch.object(self.profile,'_client',self.base/'ap1-native-client'),patch.object(self.profile.subprocess,'Popen',side_effect=spawn),patch.object(self.profile.inherited,'controlled_environment',return_value={}),patch.object(self.profile.inherited,'process_identity',return_value={'pid':321,'start_ticks':77}),patch.object(self.profile.inherited,'cleanup_process',return_value={'owned_descendants_zero':True,'process_group_empty':True}) as cleanup,patch.object(self.profile.diagnostic,'supervise',side_effect=windows):
            with self.assertRaisesRegex(ValueError,'original supervisor failure'):
                self.actual_profile_supervise(environment,mode=contract.MODE,profile=self.profile,checkpoint=lambda stage,available=None,error=None:history.append((stage,available,error)))
            cleanup.assert_called_once_with(child,[(321,77)])
        self.assertFalse(history[0][1]['cleanup']['process_group_empty'])
        last=history[-1][1];self.assertTrue(last['cleanup']['process_group_empty'])
        self.assertEqual(last['records'][0]['detail'],'original failure');self.assertEqual(len(last['caller']['records']),10)
    def test_ap1_reporting_failure_retains_samples_and_cleanup(self):
        with patch.object(self.profile,'validate_summary',side_effect=ValueError('local reporting rejection')):
            self.execute_cli()
        self.assertFalse(self.rendered);self.assertEqual(self.retire.call_count,1)
        doc=self.transaction()['observation']['payload']['document']
        self.assertEqual(doc['cleanup'],'COMPLETE');self.assertIn('local reporting rejection',json.dumps(doc))
        self.assertEqual(doc['summary']['troubleshooting']['observation']['caller']['records'][1]['input_bits'],caller()['records'][1]['input_bits'])
    def test_ap1_fresh_command_retains_and_renders_actual_words(self):
        self.assertEqual(self.execute_cli(['--preflight-only'])['state'],'ready');self.assertEqual(self.launches,0)
        receipt=self.execute_cli();self.assertTrue(receipt['renderer_completed'],self.transaction().get('observation',{}).get('payload',{}).get('document',{}).get('summary',{}))
        self.assertEqual(self.rendered[0]['admitted_result']['document']['summary']['comparison']['samples_compared'],1344)
        self.execute_cli();self.assertEqual(self.launches,1)
    def test_ap1_original_words_survive_normalization_and_lost_ack(self):
        original=self.ap1_scan.side_effect
        def bad(*args,**kwargs):
            r=original(*args,**kwargs);r['caller']['records'][1]['output_bits'][0][1]=0;return r
        self.ap1_scan.side_effect=bad;self.local_worker.lose_ack=True
        self.execute_cli();self.execute_cli();self.assertEqual(self.launches,1);self.assertEqual(self.retire.call_count,1);self.assertFalse(self.rendered)
        root=self.remote_proof/'acceptance/ACCEPTANCE_CANDIDATE'/self.delegation['execution_identity']/self.current_reservation
        record=json.loads((root/'checkpoint.json').read_bytes());self.assertLess((root/'checkpoint.json').stat().st_size,256*1024)
        self.assertEqual(record['observation']['summary']['troubleshooting']['observation']['caller']['records'][1]['output_bits'][0][1],0)
        self.assertEqual(record['observation']['cleanup'],'COMPLETE')
def load_tests(loader,tests,pattern):return unittest.TestSuite(AP1ExecutionTests(n) for n in dir(AP1ExecutionTests) if n.startswith('test_ap1_'))

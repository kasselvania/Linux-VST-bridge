import test_support
import copy,hashlib,json,unittest
from comparison import compare
from identity import envelope,hashed
from test_fixtures import fixture_manifest
from test_environment import rows as environment_rows
from test_report import rows as capability_rows

def proofs():
    result=[]
    for i,mode in enumerate(('baseline','unix_override','restored')):
        cap=[]
        source=capability_rows()
        selected=source[4:8] if i==1 else source[:4]
        for m in ('baseline','unavailable','restored'):
            for row in selected:
                old='unavailable' if i==1 else 'baseline'
                cap.append(row.replace('mode='+old,'mode='+m).replace('status=2 ','status=126 ').rsplit('tick=',1)[0]+'tick='+str(len(cap)))
        op=str(i)*32
        result.append({'session_mode':mode,'payload_sha256':'a'*64,'runner_id':'pinned','staged_runtime_sha256':'b'*64,'binding':{'operation':op,'status':'bound','artifact_sha256':'a'*64},'outer_exit':0,'cleanup_confirmed':True,'owned_survivors':0,'scratch_prefix_removed':True,'windows_environment_lines':environment_rows(),'oracle_lines':cap,'unix_environment':{'operation':op,'fixture_sha256':'a'*64,'phase':'target_runner_after_prefix_initialization','mode':'baseline','present':False,'setting':'powershell.exe=' if i==1 else 'inherited','value_bytes':15 if i==1 else 0,'sha256_utf8':hashlib.sha256(('powershell.exe=' if i==1 else '').encode()).hexdigest()}})
    identity=envelope(fixture_manifest(), '9'*64)
    for p in result:
        p['schema']=2;p['identity']=json.loads(json.dumps(identity))
        p['payload_sha256']=identity['payload']['sha256'];p['binding']['artifact_sha256']=p['payload_sha256'];p['binding']['size']=identity['payload']['size']
        p['unix_environment']['fixture_sha256']=p['payload_sha256']
        p['staged_runtime_sha256']=identity['sources']['session.py']
        p['powershell_images']=copy.deepcopy(identity['powershell_images'])
        p['source_sha256']={k:identity['sources'][k] for k in ('run.py','report.py','capability.cpp')}
        p['installed_artifacts']={k:v['sha256'] for k,v in identity['installed']['artifacts'].items() if k!='operator_frontend'}
    for i,p in enumerate(result):
        selected='intentionally_unavailable' if i==1 else 'inherited'
        before={'present':False,'length':0,'sha256':hashlib.sha256(b'').hexdigest()}
        p['policy_owner_digests']={k:p['identity']['sources'][f] for k,f in [('manager','policy-owner'),('supervisor','session.py'),('ownership','ownership.py')]}
        p['policy']={'schema':1,'operation':p['binding']['operation'],'phase':'target_runner_after_prefix_initialization',
            'environment':p['binding']['operation'],'environment_revision':1,'software_sha256':'f'*64,
            'requested':{'windows_scripting':{'powershell':selected}},'effective':{'windows_scripting':{'powershell':selected}},
            'before':before,'after':{'present':True,'length':15,'sha256':hashlib.sha256(b'powershell.exe=').hexdigest()} if i==1 else before}
    return result
class ComparisonTests(unittest.TestCase):
    def test_behavior_and_delivery_separate(self):self.assertEqual(compare(proofs())['disposition'],'IS4_OPERATION_SCOPED_HONEST_ABSENCE_PROVED')
    def test_request_alone_cannot_prove_absence(self):
        p=proofs();p[1]['oracle_lines']=p[0]['oracle_lines']
        self.assertEqual(compare(p)['disposition'],'IS4_UNIX_UNAVAILABILITY_NOT_ESTABLISHED')
    def test_wrong_epoch_artifact_cleanup_order_refused(self):
        for change in (lambda p:p[1]['binding'].update(operation=p[0]['binding']['operation']),lambda p:p[1].update(payload_sha256='c'*64),lambda p:p[1].update(owned_survivors=1),lambda p:p.reverse()):
            p=copy.deepcopy(proofs());change(p)
            with self.assertRaises(ValueError):compare(p)

    def test_every_behavior_identity_leaf_drift_refuses(self):
        def leaves(v,path=()):
            if isinstance(v,dict):
                for k,x in v.items():yield from leaves(x,path+(k,))
            elif isinstance(v,list):
                for k,x in enumerate(v):yield from leaves(x,path+(k,))
            else:yield path,v
        for path,old in leaves(proofs()[1]['identity']):
            with self.subTest(field=path):
                p=proofs();node=p[1]['identity']
                for k in path[:-1]:node=node[k]
                node[path[-1]]=not old if isinstance(old,bool) else old+1 if isinstance(old,int) else ('0' if old[0]!='0' else '1')+old[1:]
                changed=dict(leaves(p[1]['identity']));original=dict(leaves(proofs()[1]['identity']))
                self.assertEqual(sum(changed[k]!=v for k,v in original.items()),1)
                with self.assertRaises(ValueError):compare(p)
    def test_missing_extra_identity_and_outer_environment_refuse(self):
        for key in proofs()[0]['identity']:
            p=proofs();del p[1]['identity'][key]
            with self.assertRaises(ValueError):compare(p)
        p=proofs();p[1]['identity']['unknown']='x'
        with self.assertRaises(ValueError):compare(p)
        p=proofs();p[2]['windows_environment_lines']=[r.replace('length=7','length=8') for r in p[2]['windows_environment_lines']]
        with self.assertRaises(ValueError):compare(p)

    def test_production_binding_and_effective_policy_drift_refuse(self):
        for field,bad in [('operation','a'*32),('software_sha256','0'*64),('requested',{}),('effective',{}),('after',{})]:
            p=proofs();p[1]['policy'][field]=bad
            with self.assertRaises(ValueError):compare(p)
        p=proofs();p[1]['policy_owner_digests']['manager']='0'*64
        with self.assertRaises(ValueError):compare(p)

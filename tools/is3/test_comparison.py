import copy,hashlib,unittest
from comparison import compare
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
        result.append({'session_mode':mode,'payload_sha256':'a'*64,'runner_id':'pinned','staged_runtime_sha256':'b'*64,'binding':{'operation':op,'status':'bound','artifact_sha256':'a'*64},'outer_exit':0,'cleanup_confirmed':True,'owned_survivors':0,'scratch_prefix_removed':True,'windows_environment_lines':environment_rows(),'oracle_lines':cap,'unix_environment':{'operation':op,'fixture_sha256':'a'*64,'phase':'target_runner_after_prefix_initialization','mode':mode,'present':i==1,'setting':'powershell.exe=' if i==1 else 'inherited','value_bytes':15 if i==1 else 0,'sha256_utf8':hashlib.sha256(('powershell.exe=' if i==1 else '').encode()).hexdigest()}})
    return result
class ComparisonTests(unittest.TestCase):
    def test_behavior_and_delivery_separate(self):self.assertEqual(compare(proofs())['disposition'],'IS3_OPERATION_SCOPED_HONEST_ABSENCE_PROVED')
    def test_request_alone_cannot_prove_absence(self):
        p=proofs();p[1]['oracle_lines']=p[0]['oracle_lines']
        self.assertEqual(compare(p)['disposition'],'IS3_UNIX_UNAVAILABILITY_NOT_ESTABLISHED')
    def test_wrong_epoch_artifact_cleanup_order_refused(self):
        for change in (lambda p:p[1]['binding'].update(operation=p[0]['binding']['operation']),lambda p:p[1].update(payload_sha256='c'*64),lambda p:p[1].update(owned_survivors=1),lambda p:p.reverse()):
            p=copy.deepcopy(proofs());change(p)
            with self.assertRaises(ValueError):compare(p)

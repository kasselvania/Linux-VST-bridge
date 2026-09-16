import pathlib,sys,unittest
sys.path.insert(0,str(pathlib.Path(__file__).parent))
from report import summarize

def rows():
    out=[]
    for mode in ('baseline','unavailable','restored'):
        for stage in ('side_effect','capability','empty_query','fallback'):
            missing=mode=='unavailable'
            launched=int((stage=='fallback')==missing)
            status=43 if missing and stage=='fallback' else 2 if missing else 0
            out.append(f'IS3_CAP_V1 mode={mode} stage={stage} launched={launched} exited={launched} status={status} side_effect=0 tick={len(out)}')
    return out
class ReportTests(unittest.TestCase):
    def test_false_success_and_unavailable_fallback_stay_separate(self):
        r=summarize(rows());m=r['modes']
        self.assertEqual(m['baseline']['script_behavior'],'false_success')
        self.assertEqual(m['unavailable']['script_behavior'],'launch_or_execution_refused')
        self.assertTrue(m['unavailable']['source_owned_fallback_executed'])
        self.assertFalse(m['baseline']['empty_query_correct'])
        self.assertEqual(r['vendor_fallback'],'not_observed')
        self.assertEqual(r['genuine_managed_interpreter'],'not_tested')
    def test_requested_unavailability_that_still_runs_stub_is_not_success(self):
        r=rows()
        for i in range(4,8):
            stage=('side_effect','capability','empty_query','fallback')[i-4]
            launched=int(stage!='fallback')
            r[i]=f'IS3_CAP_V1 mode=unavailable stage={stage} launched={launched} exited={launched} status=0 side_effect=0 tick={i}'
        m=summarize(r)['modes']['unavailable']
        self.assertEqual(m['script_behavior'],'false_success')
        self.assertFalse(m['source_owned_fallback_executed'])
    def test_zero_exit_never_proves_script(self):
        r=rows();r[0]=r[0].replace('side_effect=0','side_effect=1')
        self.assertEqual(summarize(r)['modes']['baseline']['script_behavior'],'unavailable_observation')
    def test_side_effect_requires_requested_exit(self):
        r=rows();r[0]=r[0].replace('status=0','status=37').replace('side_effect=0','side_effect=1')
        self.assertEqual(summarize(r)['modes']['baseline']['script_behavior'],'script_side_effect_and_exit_verified')
    def test_incomplete_duplicate_reordered_and_unbounded_refused(self):
        base=rows()
        for r in (base[:-1],base+[base[0]],base[:1]+base[:1]+base[2:],['x'*257]+base[1:]):
            with self.assertRaises(ValueError):summarize(r)
    def test_wrong_fallback_exit_refused(self):
        r=rows();r[7]=r[7].replace('status=43','status=0')
        with self.assertRaises(ValueError):summarize(r)
    def test_timeout_does_not_select_fallback_or_claim_unavailable(self):
        r=rows();r[0]=r[0].replace('exited=1','exited=0').replace('status=0','status=252')
        self.assertEqual(summarize(r)['modes']['baseline']['script_behavior'],'unavailable_observation')
if __name__=='__main__':unittest.main()

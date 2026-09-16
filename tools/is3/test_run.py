import pathlib,tempfile,unittest
from unittest.mock import patch
from types import SimpleNamespace
from identity import atomic_new
from run import retire_unit
class RetirementTests(unittest.TestCase):
    def test_completed_receipt_waits_without_stop(self):
        with tempfile.TemporaryDirectory() as t:
            p=pathlib.Path(t)/'result';atomic_new(p,{'cleanup_confirmed':True,'owned_live':0})
            with patch('run.subprocess.run',side_effect=[SimpleNamespace(returncode=0),SimpleNamespace(returncode=3)]) as call,patch('run.time.sleep'):
                retire_unit('exact-unit',p,True)
            self.assertTrue(all('stop' not in c.args[0] for c in call.call_args_list))
    def test_collection_race_still_requires_positive_cleanup(self):
        for clean in (True,False):
            with tempfile.TemporaryDirectory() as t:
                p=pathlib.Path(t)/'result';atomic_new(p,{'cleanup_confirmed':clean,'owned_live':0})
                replies=[0,5,3,3]
                with patch('run.subprocess.run',side_effect=[SimpleNamespace(returncode=x) for x in replies]):
                    if clean:retire_unit('exact-unit',p,False)
                    else:
                        with self.assertRaises(RuntimeError):retire_unit('exact-unit',p,False)

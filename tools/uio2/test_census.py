import copy
from pathlib import Path
import sys
import unittest
from unittest.mock import Mock
sys.path.insert(0,str(Path(__file__).resolve().parent))
from desktop import SurfaceGraphX11
from x11 import X11OperationError
from popup import bind
from test_popup import E,graph

class Tests(unittest.TestCase):
    def test_whole_attempt_discarded_before_fresh_absent_or_hidden_state(self):
        for hidden in (False,True):
            old=graph();old['interval_ns']=[1,2];original=copy.deepcopy(old)
            fresh=copy.deepcopy(old)
            if hidden:fresh['windows'][1]['visible']=0
            else:fresh['windows'].pop(1)
            x=SurfaceGraphX11.__new__(SurfaceGraphX11)
            def facts(xid):
                if xid==91:raise X11OperationError([(3,3,0)])
                return {'xid':xid}
            x.facts=Mock(side_effect=facts);source=Mock(side_effect=[old,fresh])
            result=x.snapshot(source)
            self.assertEqual(source.call_count,2)
            self.assertEqual(result['discarded_attempts'],1)
            self.assertNotIn('91',result['x11'])
            self.assertEqual(old,original) # failed attempt cannot mutate prior graph
            with self.assertRaises(RuntimeError):bind(result,E,E)
            self.assertFalse(x.buttons if hasattr(x,'buttons') else False)

    def test_same_visible_unverifiable_window_exhausts_exact_bound(self):
        g=graph();g['interval_ns']=[1,2]
        x=SurfaceGraphX11.__new__(SurfaceGraphX11)
        x.facts=Mock(side_effect=X11OperationError([(3,3,0)]))
        source=Mock(return_value=g)
        with self.assertRaisesRegex(RuntimeError,'3 complete attempts'):x.snapshot(source)
        self.assertEqual(source.call_count,3)
        self.assertNotIn('complete',g)

    def test_other_errors_and_mixed_errors_never_retry(self):
        for err in (RuntimeError('shape unavailable'),X11OperationError([(8,3,0)]),X11OperationError([(3,3,0),(8,3,0)])):
            g=graph();g['interval_ns']=[1,2]
            x=SurfaceGraphX11.__new__(SurfaceGraphX11);x.facts=Mock(side_effect=err)
            source=Mock(return_value=g)
            with self.assertRaises(RuntimeError):x.snapshot(source)
            self.assertEqual(source.call_count,1)

    def test_terminal_change_retained_in_fresh_result(self):
        g=graph();g['interval_ns']=[1,2]
        x=SurfaceGraphX11.__new__(SurfaceGraphX11);x.facts=lambda xid: {'xid':xid}
        result=x.snapshot(lambda:g,Mock(side_effect=[False,True]))
        self.assertTrue(result['terminal']);self.assertEqual(result['discarded_attempts'],0)

if __name__=='__main__':unittest.main()

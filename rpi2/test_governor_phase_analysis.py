import json
from pathlib import Path
import tempfile
import unittest
from governor_phase_analysis import blocks, coverage


def block(position, request):
    common = dict(event='rpi1_phase', callback_sequence=1, bridge_position=position,
                  recorded_ns=9999999999, detail=0)
    specs = [('request_published',100,request,256),
             ('worker_request_observed',120,request,1),
             ('worker_process_begin',125,125,20),
             ('worker_validated',150,150,20),
             ('worker_result_published',155,0,0)]
    return [dict(common, phase=p,monotonic_ns=t,value_1=a,value_2=b,
                 detail=3 if p=='worker_request_observed' else 0) for p,t,a,b in specs]


class PhaseTests(unittest.TestCase):
    def analyze(self, rows):
        with tempfile.TemporaryDirectory() as d:
            path=Path(d)/'phase.jsonl'
            path.write_text(''.join(json.dumps(row)+'\n' for row in rows))
            return blocks(path,90,200)

    def test_same_callback_keeps_distinct_positions_and_uses_converted_time(self):
        result,published,drops=self.analyze(block(0,1)+block(256,2))
        self.assertEqual(len(published),2)
        self.assertEqual([r['position'] for r in result],[0,256])
        self.assertEqual(result[0]['queue_ms'],20/1e6)
        self.assertEqual(result[0]['service_ms'],25/1e6)
        self.assertEqual(result[0]['vendor_ms'],20/1e6)
        self.assertEqual(drops,[])

    def test_matching_position_does_not_excuse_wrong_request_id(self):
        rows=block(0,1);rows[1]['value_1']=2
        with self.assertRaisesRegex(ValueError,'identity mismatch'): self.analyze(rows)

    def test_missing_completion_not_counted_as_rendered(self):
        result,published,_=self.analyze(block(0,1)[:-1])
        self.assertEqual(len(published),1)
        self.assertEqual(result,[])

    def test_missing_middle_position_prevents_coverage_claim(self):
        queued=[dict(bridge_position=i*256,value_2=256) for i in range(10)]
        self.assertTrue(coverage(queued,queued,10*256/48000,[])['complete'])
        missing=queued[:5]+queued[6:]
        result=coverage(missing,missing,10*256/48000,[])
        self.assertFalse(result['complete'])
        self.assertEqual(result['missing_positions_inside_span'],1)
        self.assertFalse(coverage(queued,queued[:-1],10*256/48000,[])['complete'])


if __name__=='__main__':unittest.main()

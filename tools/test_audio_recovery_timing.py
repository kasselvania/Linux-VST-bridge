import json
import pathlib
import unittest
from audio_recovery_timing import attribute


class TimingAttributionTests(unittest.TestCase):
    def witness(self):
        path=pathlib.Path(__file__).resolve().parents[1]/'evidence/audio-recovery/2026-10-02-render-preemption-witness.json'
        return json.loads(path.read_text())

    def test_physical_request_was_preempted_across_its_deadline_inside_sdk(self):
        result=attribute(self.witness())
        self.assertEqual(result['preemption_lower_bound_ns'],6_000_040)
        self.assertEqual(result['preemptions_inside_sdk_across_deadline'],
            [[8235059029728,8235065029768,True]])

    def test_stopped_capture_is_not_evidence_of_no_preemption(self):
        witness=self.witness();witness['switches']=witness['switches'][:3]
        with self.assertRaisesRegex(ValueError,'does not cover'):attribute(witness)

    def test_lost_or_unpaired_switches_refuse_attribution(self):
        witness=self.witness();witness['lost_events']=1
        with self.assertRaises(ValueError):attribute(witness)
        witness=self.witness();witness['switches'][2][1]='OUT'
        with self.assertRaisesRegex(ValueError,'missing switch in'):attribute(witness)

    def test_clock_bound_uncertainty_does_not_become_an_exact_cpu_time(self):
        witness=self.witness();witness['clock_uncertainty_ns']=4_000_000
        witness['switches'].insert(0,[8235040000000,'OUT',False])
        witness['switches'].append([8235080000000,'IN',False])
        result=attribute(witness)
        self.assertLess(result['preemption_lower_bound_ns'],6_000_040)
        self.assertEqual(result['preemptions_inside_sdk_across_deadline'],[])


if __name__=='__main__':unittest.main()

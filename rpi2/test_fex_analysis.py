import unittest
from fex_stats import COUNTERS
from fex_analysis import reduce_trial


def trial(tids):
    samples=[]
    for i,tid in enumerate(tids):
        counters=dict.fromkeys(COUNTERS,0);counters['jit_count']=i*5
        samples.append({'fex':{'begin_ns':i*1000000000,'end_ns':i*1000000000+10,
            'observer_cpu_ns':10,'stats':{'rows':[{'slot_offset':64,'windows_tid':tid,'counters':counters}]}}})
    return {'name':'synthetic','graph_observed_ns':0,'samples':samples}


class CounterWindows(unittest.TestCase):
    def test_empty_or_single_sample_is_unavailable(self):
        for ids in [[],[1]]:
            self.assertIn('unavailable',reduce_trial(trial(ids)))

    def test_edge_intervals_and_churn_are_not_exact_window_totals(self):
        result=reduce_trial(trial([1,1,1,2]))
        idle=result['windows']['idle']
        self.assertEqual(idle['contained_counter_deltas']['jit_count'],0)
        self.assertEqual(idle['overlapping_stable_counter_deltas']['jit_count'],10)
        self.assertEqual(result['unstable_intervals'],1)
        self.assertEqual(result['windows']['attack']['contained_counter_deltas']['jit_count'],0)


if __name__=='__main__':unittest.main()

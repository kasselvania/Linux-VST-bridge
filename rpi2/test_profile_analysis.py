import tempfile
import unittest
from pathlib import Path

from profile_analysis import aligned_work, annotate_map_candidates, map_ambiguity, parse_samples, profile_window


class CriticalProfileReduction(unittest.TestCase):
    def test_weighted_jit_owner_and_block_without_private_path(self):
        with tempfile.TemporaryDirectory() as temp:
            path=Path(temp)/'samples.txt'
            path.write_text(
                ' lvb-audio 1/10 10.100000000: 100 7f0010 '
                '\\??\\c:\\private\\pigmentsprocessor.dll+0x735530 (0x7f0000) (/tmp/perf-1.map)\n'
                ' lvb-audio 1/10 10.200000000: 1 7f0020 [unknown] (/tmp/perf-1.map)\n'
                ' lvb-audio 1/10 11.000000000: 10 7f0030 foo (/usr/lib/libc.so.6)\n')
            rows=parse_samples(path)
            result=profile_window(rows,10_000_000_000,11_000_000_000)
            self.assertEqual(result['samples'],2)
            self.assertEqual(result['perf_selected_owners_percent']['pigmentsprocessor.dll'],99.01)
            self.assertEqual(result['perf_selected_guest_blocks'][0]['image_rva'],'0x735530')
            self.assertNotIn('private',str(result))

    def test_overlapping_rvas_preserve_module_but_cross_module_is_withheld(self):
        with tempfile.TemporaryDirectory() as temp:
            path=Path(temp)/'map'
            path.write_text('0x100 20 private.dll+0x10 (0x100)\n'
                            '0x100 20 private.dll+0x20 (0x100)\n'
                            '0x110 20 other.dll+0x30 (0x110)\n'
                            'malformed\n')
            rows=[dict(ns=10,ip=0x105,period=3,jit_map=True,owner='private.dll'),
                  dict(ns=11,ip=0x115,period=1,jit_map=True,owner='private.dll')]
            self.assertEqual(annotate_map_candidates(path,rows),dict(valid_lines=3,malformed_lines=1,duplicate_starts=1))
            result=map_ambiguity(rows,0,20)
            self.assertEqual(result['multiple_guest_rva_percent'],100.0)
            self.assertEqual(result['cross_module_ambiguous_percent'],25.0)
            self.assertEqual(result['unambiguous_module_lower_bound_percent_of_all_user_cycles']['private.dll'],75.0)

    def test_refuse_malformed_perf_line(self):
        with tempfile.TemporaryDirectory() as temp:
            path=Path(temp)/'samples.txt';path.write_text('not a perf sample\n')
            with self.assertRaisesRegex(ValueError,'line 1'):
                parse_samples(path)

    def test_cpu_denominator_uses_completed_frames_and_exposes_read_delay(self):
        def snapshot(ns,frames,delivered,missing,caller,worker):
            return dict(monotonic_ns=ns,counter_read_begin_ns=ns+20_000_000,
                        counter_read_end_ns=ns+120_000_000,
                        current_counters=dict(bridge_processed_frames=frames,
                                              delivered_frames=delivered,missing_frames=missing,gaps=2),
                        threads={'3:10':dict(cpu_ns=caller,wait_ns=0,user_ticks=10,
                                             system_ticks=1,minor_faults=0,major_faults=0),
                                 '3:11':dict(cpu_ns=worker,wait_ns=10_000_000,user_ticks=5,
                                             system_ticks=0,minor_faults=0,major_faults=0)})
        trial=dict(graph_observed_ns=0,profile={'linux_pid':3},samples=[
            snapshot(3_900_000_000,0,0,0,0,0),
            snapshot(4_100_000_000,2560,2048,512,1_000_000_000,500_000_000),
            snapshot(13_800_000_000,5120,4096,512,2_000_000_000,900_000_000)])
        result=aligned_work(trial,4,14,10,11)
        self.assertEqual(result['completed_calls'],10)
        self.assertEqual(result['missing_frames'],0)
        self.assertEqual(result['caller']['cpu_ms_per_completed_256_frames'],100.0)
        self.assertEqual(result['worker']['cpu_ms_per_completed_256_frames'],40.0)
        self.assertEqual(result['first']['status_read_end_seconds_after_graph'],4.22)


if __name__=='__main__':
    unittest.main()

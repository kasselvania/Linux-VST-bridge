import pathlib
import struct
import tempfile
import unittest
import process_call_timing as timing


class Timing(unittest.TestCase):
    def binary(self, rows, **changes):
        summary = dict(schema=1, size=112, capacity=262144, namespace_pid=12, record_size=112,
                       flags=15, instance=7, offered=len(rows), retained=len(rows), capacity_dropped=0,
                       contention_dropped=0, allocation_dropped=0, invalid_clocks=0, invalid_identity=0,
                       after_seal=0, unfinished_writers=0, sequence_overflow=0)
        summary.update(changes)
        return b'LVBPC001'+struct.pack('<II', 128, 112)+timing.SUMMARY.pack(
            *(summary[key] for key in timing.SUMMARY_FIELDS))+b''.join(timing.RECORD.pack(*row) for row in rows)

    def row(self, sequence, n, elapsed, result=0, outcome=1):
        return timing.Call(1,112,7,sequence,100+sequence*10000000,100+sequence*10000000+elapsed,
                           9,1,48000.,0,12,13,n,0,0,512,4,result,111 if outcome == 1 else 47,outcome)

    def read_bytes(self, data):
        with tempfile.TemporaryDirectory() as directory:
            path=pathlib.Path(directory)/'calls.bin';path.write_bytes(data)
            return timing.read(path)

    def test_every_exit_actual_n_and_independent_cadence(self):
        rows=[self.row(0,0,100000000),self.row(1,64,1333334,1),self.row(2,1,2000,outcome=2)]
        summary, decoded=self.read_bytes(self.binary(rows));self.assertEqual(decoded,rows)
        result=timing.analyze(summary,decoded)
        self.assertEqual(result['retained_calls'],3)
        self.assertEqual(result['cadence_exceeding_sequences'],[1])
        self.assertEqual(result['cadence_undefined_calls'],1)
        self.assertEqual(result['outcomes'],{'1':2,'2':1})
        self.assertEqual(result['sdk_results'],{'0':1,'1':1})

    def test_no_filter_on_failed_or_crossing_call(self):
        rows=[self.row(0,1,30000000,1),self.row(1,512,1)]
        summary, decoded=self.read_bytes(self.binary(rows));result=timing.analyze(summary,decoded,100,1000)
        self.assertEqual(result['selected_entry_calls'],1)
        self.assertEqual(result['window']['crossing_end_calls'],1)
        self.assertEqual(result['durations']['max_ns'],30000000)

    def test_extent_and_false_coverage_refuse(self):
        data=self.binary([self.row(0,64,1)])
        for bad in (data[:-1],data+b'x',self.binary([self.row(0,64,1)],capacity_dropped=1),
                    self.binary([self.row(1,64,1)]),self.binary([self.row(0,64,1)],unfinished_writers=1)):
            with self.assertRaises(ValueError):self.read_bytes(bad)

    def test_partial_and_sequence_wrap_are_explicit(self):
        summary,rows=self.read_bytes(self.binary([self.row(0,1,1)],flags=7,offered=2,capacity_dropped=1))
        self.assertFalse(timing.analyze(summary,rows)['record_population_complete'])
        with self.assertRaises(ValueError):self.read_bytes(self.binary([self.row(0,1,1),self.row(0,1,1)],flags=7))

    def test_readable_population_does_not_invent_export_success(self):
        import json
        summary,rows=self.read_bytes(self.binary([self.row(0,64,1)]))
        self.assertTrue(timing.analyze(summary,rows)['record_population_complete'])
        self.assertIsNone(timing.analyze(summary,rows)['dataset_complete'])
        with tempfile.TemporaryDirectory() as directory:
            report=pathlib.Path(directory)/'lifecycle.ndjson'
            event=dict(summary,event='native_process_call_summary',export_status=3)
            report.write_text(json.dumps(event)+'\n')
            confirmed=timing.export_custody(report,summary)
            self.assertFalse(confirmed)
            self.assertFalse(timing.analyze(summary,rows,export_confirmed=confirmed)['dataset_complete'])
            event['export_status']=0;report.write_text(json.dumps(event)+'\n')
            self.assertTrue(timing.export_custody(report,summary))
            event['offered']=2;report.write_text(json.dumps(event)+'\n')
            with self.assertRaises(ValueError):timing.export_custody(report,summary)


if __name__ == '__main__':unittest.main()

import copy, pathlib, sys, unittest
sys.path.insert(0,str(pathlib.Path(__file__).parent))
from ap0_contract import *

def blocks():
    return [dict(block=b,frames=16,sample_rate=48000,gain=.5 if b==0 else .25,
        sample_format='kSample32',process_mode='kOffline',process_result=0,worker_thread=True,
        input_silence_flags=3 if b==2 else 0,output_silence_flags=3 if b==2 else 0,
        input_bits=[[GUARD]+[bits(v) for v in inputs(b,c)]+[GUARD] for c in range(2)],
        output_bits=[[GUARD]+[bits(v*(.5 if b==0 else .25)) for v in inputs(b,c)]+[GUARD] for c in range(2)]) for b in range(3)]

class AP0Tests(unittest.TestCase):
    def test_independent_every_sample(self):
        self.assertEqual(compare_blocks(blocks())['maximum_absolute_error'],0)
        self.assertEqual(compare_blocks(blocks())['samples_compared'],96)
        for b in range(3):
            for c in range(2):
                for i in range(1,17):
                    data=blocks();data[b]['output_bits'][c][i]=bits(7)
                    with self.assertRaises(ValueError):compare_blocks(data)
    def test_reject_corrupt_buffers(self):
        for mutate in [lambda d:d[0]['output_bits'].reverse(),
                       lambda d:d[1].update(output_bits=d[0]['output_bits']),
                       lambda d:d[0].update(output_bits=d[0]['input_bits']),
                       lambda d:d[2]['output_bits'][0].__setitem__(1,0x7fc12345),
                       lambda d:d[0]['input_bits'][0].__setitem__(2,bits(7)),
                       lambda d:d[1]['output_bits'][1].__setitem__(0,0),
                       lambda d:d[0]['output_bits'][0].pop(),
                       lambda d:d[0].update(worker_thread=False)]:
            data=blocks();mutate(data)
            with self.assertRaises(ValueError):compare_blocks(data)
    def test_host_order_and_preallocation(self):
        source=(pathlib.Path(__file__).parents[1]/'windows-factory-probe/source/offline_processing.cpp').read_text()
        self.assertLess(source.index('addParameterData'),source.index('set_active_true'))
        self.assertLess(source.index('worker.join()'),source.index('set_active_false'))
        self.assertLess(source.index('set_processing_false'),source.index('worker.join()'))
        process=source[source.index('block.result=processor.process'):source.index('ap0_process_completed')]
        self.assertNotIn('new ',process)

if __name__=='__main__': unittest.main()

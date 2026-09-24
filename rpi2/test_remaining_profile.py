import struct
import tempfile
import unittest
from pathlib import Path

from recorder_compare import outstanding_frames
from remaining_analysis import kernel_events,kernel_window
from remaining_profile import FAMILY,SEGMENT_BYTES,family_blocks,map_candidates


class RemainingProfileProof(unittest.TestCase):
    def test_live_status_gap_is_not_reported_as_quantum_change(self):
        with self.assertRaisesRegex(ValueError,'statistics unavailable'):
            outstanding_frames({'callbacks':10,'paused_frames':0},256)
        with self.assertRaisesRegex(ValueError,'quantum differs'):
            outstanding_frames({'processing_quantum':512,'bridge_processed_frames':256,
                                'callbacks':10,'paused_frames':0},256)

    def test_map_clusters_and_exact_fex_tail_extent(self):
        map_bytes=(b'0x100300 d20 pigmentsprocessor.dll+0x735530 (0x100300)\n'
                   b'0x100304 d20 pigmentsprocessor.dll+0x735530 (0x100304)\n'
                   b'0x200300 d20 pigmentsprocessor.dll+0x735530 (0x200300)\n')
        self.assertEqual(map_candidates(map_bytes),[(0x100300,0xd20),(0x200300,0xd20)])
        module=0x180000000
        blob=bytearray(SEGMENT_BYTES)
        block=0x300;tail=block+0xc00
        struct.pack_into('<I',blob,block,0xc00)
        struct.pack_into('<QQQIIIB',blob,tail,0xd20,module+FAMILY[1],0x700,1,40,0,0)
        blob[tail+40]=1 # one-byte packed RIP entry is valid
        found=family_blocks(blob,0x100000,module)
        self.assertEqual([(x['begin'],x['end']) for x in found],[(0x100300,0x101020)])
        struct.pack_into('<Q',blob,tail+8,module+0x734000)
        self.assertEqual(family_blocks(blob,0x100000,module),[])

    def test_kernel_stack_periods_and_thread_identity(self):
        data=('Processing Thre   7/8   12.100000: 100 \n'
              '\tffff getrusage ([kernel.kallsyms])\n\n'
              'Processing Thre   7/8   12.200000: 300 \n'
              '\tffff do_sched_yield ([kernel.kallsyms])\n')
        with tempfile.TemporaryDirectory() as temp:
            file=Path(temp)/'samples';file.write_text(data)
            events=kernel_events(file,8)
            result=kernel_window(events,12_000_000_000,13_000_000_000)
            self.assertEqual(result['samples'],2)
            self.assertEqual(result['categories']['sched_yield']['weighted_percent'],75.0)
            with self.assertRaisesRegex(ValueError,'identity'):
                kernel_events(file,9)


if __name__=='__main__':unittest.main()

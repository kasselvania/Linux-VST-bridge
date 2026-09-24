import struct
import unittest
from fex_stats import HEADER,SLOT,decode,interval


def sample(tid=22,value=5):
    blob=bytearray(4096)
    HEADER.pack_into(blob,0,2,2,112,b'test',64,4096,0)
    SLOT.pack_into(blob,64,0,tid,*([value]*12),0)
    return blob


class StatsDecoder(unittest.TestCase):
    def test_layout_and_windows_identity(self):
        parsed=decode(sample())
        self.assertEqual(parsed['rows'][0]['windows_tid'],22)
        self.assertEqual(parsed['rows'][0]['counters']['jit_count'],5)
        self.assertEqual(interval(parsed,decode(sample(value=8)))['matched_slot_deltas']['jit_count'],3)

    def test_bad_version_size_and_links_refused(self):
        cases=[]
        b=sample();b[0]=3;cases.append(b)
        b=sample();struct.pack_into('<I',b,56,8192);cases.append(b)
        b=sample();struct.pack_into('<I',b,52,65);cases.append(b)
        b=sample();struct.pack_into('<I',b,64,64);cases.append(b)
        for b in cases:
            with self.assertRaises(ValueError):decode(b)

    def test_slot_reuse_and_counter_reset_not_subtracted(self):
        old=decode(sample())
        for new in [decode(sample(tid=23)),decode(sample(value=1))]:
            result=interval(old,new)
            self.assertFalse(result['observed_identity_stable'])
            self.assertEqual(result['matched_slot_deltas']['jit_count'],0)
        self.assertEqual(decode(sample(tid=0))['rows'],[])


if __name__=='__main__':unittest.main()

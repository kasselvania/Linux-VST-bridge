import array
import pathlib
import struct
import sys
import unittest
sys.path.insert(0,str(pathlib.Path(__file__).parent))
from x11 import normalized,summaries,X11
from xrecord import pointer_packet
from observe import Observer,CAPACITY,SIZE,HEADER,RECORD

class Tests(unittest.TestCase):
    def test_coordinates_follow_exact_current_window(self):
        self.assertEqual(normalized((100,200,401,301),(.5,.5)),(300,350))
        self.assertEqual(normalized((-100,0,101,101),(1,1)),(0,100))
        for point in [(1.01,0),(float('nan'),0),(-.1,.2)]:
            with self.assertRaises(ValueError):normalized((0,0,10,10),point)
    def test_local_frame_change_and_stability_are_not_semantic_claims(self):
        a=array.array('I',[0]*16*16);meta,prior=summaries(a.tobytes(),16,16,64,regions=[(0,0,8,8)])
        a[8*16+8]=0x00ff0000;b,new=summaries(a.tobytes(),16,16,64,prior,regions=[(0,0,8,8)])
        self.assertEqual(b['changed_sample_bounds'],[8,8,16,16]);self.assertEqual(b['changed_sample_percent'],25)
        self.assertEqual(meta['region_hashes'],b['region_hashes']);self.assertNotEqual(meta['sha256'],b['sha256'])
        c,_=summaries(a.tobytes(),16,16,64,new);self.assertEqual(c['changed_sample_percent'],0)
        with self.assertRaises(ValueError):summaries(b'',16,16,64)
    def test_server_receipt_is_exact_target_not_global_input(self):
        b=struct.pack('<BBHIIIIhhhhHBB',4,1,2,15,20,99,0,100,200,3,4,256,1,0)
        r=pointer_packet(b,99);self.assertEqual(r['client'],[3,4]);self.assertEqual(r['state'],256)
        self.assertIsNone(pointer_packet(b,100))
        self.assertIsNone(pointer_packet(bytes([2])+b[1:],99)) # keyboard excluded
        self.assertIsNone(pointer_packet(b[:31],99))
    def test_only_complete_append_records_survive(self):
        o=Observer.__new__(Observer);o.map=bytearray(SIZE);o.cursor=0
        o.read=lambda at:struct.unpack_from('<Q',o.map,at)[0]
        struct.pack_into('<Q',o.map,56,2)
        struct.pack_into('<Q',o.map,HEADER,1)
        self.assertEqual(len(o.take()),1)
        self.assertEqual(o.cursor,1) # second interrupted commit not read
        struct.pack_into('<Q',o.map,HEADER+RECORD,2)
        self.assertEqual(len(o.take()),1)
        struct.pack_into('<Q',o.map,56,CAPACITY+1)
        with self.assertRaises(RuntimeError):o.take()
    def test_cancellation_releases_only_owned_down_state(self):
        class Fake:
            def __init__(self):self.calls=[]
            def __getattr__(self,name):return lambda *args:self.calls.append((name,args)) or 1
        x=X11.__new__(X11);x.t=Fake();x.x=Fake();x.buttons={1};x.keys={24};x.display=1;x.pixmap=0;x.prior_handler=None
        x.close();self.assertEqual([c[0] for c in x.t.calls],['XTestFakeButtonEvent','XTestFakeKeyEvent'])
        self.assertEqual(x.buttons,set());self.assertEqual(x.keys,set())

if __name__=='__main__':unittest.main()

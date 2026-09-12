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
    def test_action_command_and_frame_metadata_have_separate_namespaces(self):
        from capture import Capture
        import tempfile,json,zlib
        from types import SimpleNamespace
        with tempfile.TemporaryDirectory() as directory:
            out=pathlib.Path(directory);command=out/'action-2.json'
            original=b'{"kind":"move","point":[0.1,0.2]}'
            command.write_bytes(original)
            cap=Capture.__new__(Capture);cap.out=out
            cap.x=SimpleNamespace(capture=lambda:({'width':1,'height':1},b'1234'))
            cap.snapshot('action-2')
            self.assertEqual(command.read_bytes(),original)
            self.assertEqual(json.loads((out/'snapshots/action-2.json').read_text()),{'width':1,'height':1})
            self.assertEqual(zlib.decompress((out/'snapshots/action-2.bgra.z').read_bytes()),b'1234')
            with self.assertRaises(FileExistsError):cap.snapshot('action-2')
            with self.assertRaises(ValueError):cap.snapshot('../action-2')
            self.assertEqual(command.read_bytes(),original)

    def test_capture_preserves_records_and_releases_views_after_process_exit(self):
        from capture import Capture
        from unittest.mock import Mock,patch
        from types import SimpleNamespace
        import tempfile,json,time
        with tempfile.TemporaryDirectory() as directory:
            cap=Capture.__new__(Capture);cap.out=pathlib.Path(directory)
            cap.record=cap.observer=cap.helper=None;cap.result={};cap.pid=123
            cap.frames=[];cap.windows=[{'message':513}];cap.gestures=[];cap.inputs=[];cap.brackets=[]
            cap.frame_dropped=0;cap.cpu_start=time.process_time_ns()
            cap.gui=SimpleNamespace(dropped=[0,0],close=Mock());cap.x=SimpleNamespace(close=Mock())
            cap.c=SimpleNamespace(fault=SimpleNamespace(snapshot=lambda:{'terminal':{'class':1}},close=Mock()))
            with patch('capture.process_sample',side_effect=FileNotFoundError):cap.close()
            r=json.loads((cap.out/'capture.json').read_text())
            self.assertTrue(r['process_after']['absent']);self.assertEqual(r['win32'],cap.windows)
            self.assertEqual(r['after'],{'terminal':{'class':1}})
            cap.gui.close.assert_called_once();cap.x.close.assert_called_once();cap.c.fault.close.assert_called_once()

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
    def test_stale_pointer_acknowledgement_never_becomes_a_click(self):
        x=X11.__new__(X11);x.expected_pointer=[110,220]
        x.check_identity=lambda:(100,200,200,200)
        x.pointer=lambda:dict(screen=[101,201],client=[1,1],active=[42],mask=0)
        with self.assertRaisesRegex(RuntimeError,'not acknowledged'):x.button(True)
    def test_diagnostic_keys_cannot_type_arbitrary_text(self):
        x=X11.__new__(X11)
        for key in ['A','password',42]:
            with self.assertRaises(ValueError):x.key(True,key)
    def test_helper_launch_cannot_select_an_arbitrary_executable(self):
        from launch import Context
        c=Context.__new__(Context)
        with self.assertRaisesRegex(RuntimeError,'unknown diagnostic'):c.helper('unreviewed.exe',[],pathlib.Path('/tmp/unused'),1)
        c.admission={'accessibility_probe_permitted':False}
        with self.assertRaisesRegex(RuntimeError,'prohibits'):c.helper('uio1-accessibility.exe',[],pathlib.Path('/tmp/unused'),1)
    def test_replaced_or_aliased_helper_bytes_refused(self):
        from launch import sealed_bytes
        import tempfile,hashlib
        with tempfile.TemporaryDirectory() as directory:
            p=pathlib.Path(directory)/'helper';p.write_bytes(b'exact');sha=hashlib.sha256(b'exact').hexdigest()
            self.assertEqual(sealed_bytes(p,sha),b'exact')
            p.write_bytes(b'changed')
            with self.assertRaises(RuntimeError):sealed_bytes(p,sha)
            link=p.with_name('alias');link.symlink_to(p)
            with self.assertRaises(OSError):sealed_bytes(link,sha)

    def test_public_projection_uses_aliases_and_closed_scalar_fields(self):
        from report import project
        import json
        private='PRIVATE_ACCOUNT_PATH_PAYLOAD'
        status=dict.fromkeys(('committed','dropped','ready','closed','hook_calls','hook_ticks',
            'max_hook_ticks','filtered','heartbeat_errors','scope_errors','unhook_errors','detached'),0)
        status['private']=private
        sample=lambda at:dict(at=at,utime=at//10,stime=0,hz=100)
        raw=dict(process_before=sample(100),process_trace_off_end=sample(200),
            process_diagnostic_idle_end=sample(300),diagnostic_python_cpu_ns=10,
            profile_fingerprint='a'*64,adapter='human',frequency=1000,
            windows=[dict(type='window',hwnd=999,tid=222,name=private)],
            inputs=[dict(kind='action_begin',action=2,interval_ns=[300,300])],
            brackets=[dict(linux_before_ns=310,linux_after_ns=320,windows_qpc=10,frequency=1000)],
            win32=[dict(action=2,source=1,qpc=10,hwnd=999,message=513,
                focus=999,active=999,capture=0,x=1,y=2,result=0,text=private)],
            gui=[],x11=[],renderer=['opengl32.dll'],observer_status=status,
            x11_dropped=0,x11_unparsed=0,gui_dropped=[0,0],frame_dropped=0,frame_interval_ms=100,
            helper_cleanup=dict(exit=0,kept=0,overflow=0,stderr_bytes=0,path=private,
                cleanup=dict(owned_descendants_zero=True,process_group_empty=True,pid=999)),
            frames=[dict(action=2,region_hashes=[str(i)*64]*3,cpu_ns=1,
                interval_ns=[330+i,340+i],capture_backend='exact-window',changed_sample_percent=i,
                pixels=private) for i in (0,1)])
        projection,facts=project(raw)
        output=json.dumps([projection,facts])
        self.assertNotIn(private,output)
        self.assertNotIn('999',output)
        witness=projection['witnesses'][0]['witness']
        self.assertEqual(witness['target_alias'],1)
        self.assertEqual(witness['focus_alias'],1)
        # This manual run starts on Play, so the FX Macro region cannot imply a
        # control redraw even though unrelated pixels in that region changed.
        self.assertFalse(any(w['witness']['kind']=='first_pixel_change' for w in projection['witnesses']))

if __name__=='__main__':unittest.main()

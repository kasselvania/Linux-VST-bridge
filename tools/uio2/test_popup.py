import copy
from dataclasses import asdict, replace
import json
from pathlib import Path
import struct
import sys
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parent))
from popup import Editor, bind, Capsule, Permit, WindowGraph, SIZE, SLOT
from desktop import unobscured

E=Editor('1'*32,1,2,3,4,50,60,70,80,90)
def row(hwnd,owner=0,root_owner=80,**kw):
    return dict(hwnd=hwnd,parent=0,owner=owner,root=hwnd,root_owner=root_owner,xid=hwnd+10,
                pid=50,tid=70,visible=1,enabled=1,minimized=0,dpi=96,style=0,reserved=0,
                rect=[100,100,300,200],client=[100,100,300,200],work=[0,0,1280,800],**kw)
def graph():return dict(editor=asdict(E),windows=[row(80),row(81,80),row(82,83,83),row(83,root_owner=83)])
class Tests(unittest.TestCase):
    def test_exact_chain_ignores_unrelated_lookalike(self):
        self.assertEqual(bind(graph(),E,E)['hwnd'],81)
        g=graph();g['windows'][1]['owner']=84;g['windows'].append(row(84,80));g['windows'][-1]['visible']=0
        self.assertEqual(bind(g,E,E)['hwnd'],81)
    def test_stale_epoch_activation_generation_session_refused(self):
        for key,val in [('epoch',5),('activation',3),('generation',4),('native_view',2),('session','2'*32),('start',61),('pid',51)]:
            with self.subTest(key=key),self.assertRaises(RuntimeError):bind(graph(),E,replace(E,**{key:val}))
    def test_wrong_owner_process_thread_and_unavailable_binding(self):
        for key,val in [('owner',83),('owner',81),('root_owner',83),('pid',51),('tid',71),('xid',0),('dpi',0),('visible',0),('enabled',0),('minimized',1),('root',80)]:
            g=graph();g['windows'][1][key]=val
            with self.subTest(key=key,val=val),self.assertRaises(RuntimeError):bind(g,E,E)
    def test_ambiguous_and_closed_popup_refused(self):
        g=graph();prior=bind(g,E,E);g['windows'].append(row(84,80))
        with self.assertRaisesRegex(RuntimeError,'ambiguous'):bind(g,E,E)
        g=graph();g['windows'].pop(1)
        with self.assertRaises(RuntimeError):bind(g,E,E,prior)
        for key,val in [('hwnd',86),('rect',[101,100,301,200]),('dpi',120),('xid',999)]:
            g=graph();g['windows'][1][key]=val
            with self.assertRaises(RuntimeError):bind(g,E,E,prior)
    def test_foreign_occlusion_and_unmapped_refused(self):
        stack=[dict(xid=1,visible=True,rect=[0,0,800,600]),dict(xid=91,visible=True,rect=[100,100,300,200])]
        unobscured(stack,91,[100,100,300,200])
        stack.append(dict(xid=999,visible=True,rect=[250,150,400,300]))
        with self.assertRaisesRegex(RuntimeError,'occlusion'):unobscured(stack,91,[100,100,300,200])
        stack[-1]['visible']=False;unobscured(stack,91,[100,100,300,200])
        stack[1]['visible']=False
        with self.assertRaises(RuntimeError):unobscured(stack,91,[100,100,300,200])
    def test_pixmap_masks_require_exact_source_visual(self):
        import ctypes as C
        from x11 import Visual,Attributes,validated_masks
        v=Visual();v.cls=4;v.red_mask=0xff0000;v.green_mask=0xff00;v.blue_mask=0xff
        a=Attributes();a.depth=24;a.visual=C.cast(C.pointer(v),C.c_void_p)
        self.assertEqual(validated_masks((0,0,0),True,24,a),(0xff0000,0xff00,0xff))
        for pixmap,depth,attrs in ((False,24,a),(True,32,a),(True,24,None)):
            with self.assertRaises(RuntimeError):validated_masks((0,0,0),pixmap,depth,attrs)
        v.cls=3
        with self.assertRaises(RuntimeError):validated_masks((0,0,0),True,24,a)

    def test_multiple_x_connections_route_errors_and_close_in_either_order(self):
        import ctypes as C
        import x11
        class Lib:
            changes=[]
            def XSetErrorHandler(self,h):self.changes.append(h);return None
        lib=Lib();a=[];b=[]
        x11._attach_errors(lib,101,a);x11._attach_errors(lib,202,b)
        e=x11.XError();e.code=8;e.major=142;e.minor=6
        x11._route_error(101,C.pointer(e))
        self.assertEqual(a,[(8,142,6)]);self.assertEqual(b,[])
        x11._detach_errors(lib,101)
        x11._route_error(202,C.pointer(e));self.assertEqual(b,[(8,142,6)])
        self.assertEqual(len(lib.changes),1)
        x11._detach_errors(lib,202);self.assertEqual(len(lib.changes),2)
        self.assertEqual(x11._ERROR_OWNERS,{})

    def capsule(self):return Capsule('generated-popup',E,bind(graph(),E,E),'resize_window',(200,150),100,1_000_000_000)
    def test_capsule_one_closed_action_not_retry_or_verdict(self):
        c=self.capsule();p=Permit(c);self.assertEqual(len(p.consume({'action':'resize_window'},E,101)),64)
        with self.assertRaisesRegex(RuntimeError,'no retries'):p.consume({'action':'resize_window'},E,102)
        for command in ({'action':'resize_window','x':200},{'action':'retry'},{'verdict':'pass'},{'shell':'ls'},{'action':'relaunch'}):
            with self.assertRaises(RuntimeError):Permit(c).consume(command,E,101)
        for kw in (dict(maximum_actions=2),dict(point=(300,150)),dict(point=(200.,150)),dict(executor='astra'),dict(forbidden=()),dict(stop=()),dict(action='escape'),dict(expires_ns=999_000_000_000)):
            with self.subTest(kw=kw),self.assertRaises(RuntimeError):replace(c,**kw).validate(E,101)
        with self.assertRaises(RuntimeError):c.validate(E,1_000_000_001)
    def test_graph_interrupted_copy_and_bounds_refused(self):
        g=WindowGraph.__new__(WindowGraph);g.editor=E;g.map=bytearray(SIZE)
        g.read=lambda n:struct.unpack_from('<Q',g.map,n)[0]
        def write(n,value):
            struct.pack_into('<Q',g.map,n,value)
            if n==48:struct.pack_into('<Q',g.map,56,value)
        g.write=write
        struct.pack_into('<Q',g.map,64,1)
        at=4096+SLOT;struct.pack_into('<QQIIQ',g.map,at,1,100,0,0,0)
        self.assertEqual(g.snapshot()['windows'],[])
        struct.pack_into('<I',g.map,at+16,129)
        with self.assertRaisesRegex(RuntimeError,'incomplete'):g.snapshot()
        struct.pack_into('<I',g.map,at+16,0);struct.pack_into('<Q',g.map,at,0)
        with self.assertRaisesRegex(RuntimeError,'changing'):g.snapshot()
    def test_popup_input_inherits_held_input_and_settlement_refusal(self):
        from x11 import X11
        class Target:
            buttons=set();window=91;expected_pointer=[200,150]
            def check_identity(self):return(100,100,200,100)
            def pointer(self):return self.p
            def active_for_input(self,p):return True
        t=Target();t.p=dict(screen=[201,150],client=[101,50],mask=0)
        with self.assertRaisesRegex(RuntimeError,'not acknowledged'):X11.button(t,True)
        t.p.update(screen=[200,150],mask=1)
        with self.assertRaisesRegex(RuntimeError,'held'):X11.button(t,True)

if __name__=='__main__':unittest.main()

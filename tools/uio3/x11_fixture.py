"""Actual libXi/XRECORD production edges on an isolated Xvfb, never user input."""
import ctypes as C
import ctypes.util
import os
import pathlib
import sys
import time
sys.path[:0]=[str(pathlib.Path(__file__).parent),str(pathlib.Path(__file__).parent.parent/'uio1')]
from xi2 import XI2,XIRecorder
from x11 import X11,U,I,P
assert os.environ.get('UIO3_XVFB_FIXTURE')=='1'
x=C.CDLL(ctypes.util.find_library('X11'))
x.XOpenDisplay.argtypes=[C.c_char_p];x.XOpenDisplay.restype=P;d=x.XOpenDisplay(None);assert d
x.XDefaultRootWindow.argtypes=[P];x.XDefaultRootWindow.restype=U;root=x.XDefaultRootWindow(d)
x.XCreateSimpleWindow.argtypes=[P,U,I,I,C.c_uint,C.c_uint,C.c_uint,U,U];x.XCreateSimpleWindow.restype=U
w=x.XCreateSimpleWindow(d,root,100,80,240,160,0,0,0)
x.XSelectInput.argtypes=[P,U,C.c_long];x.XSelectInput(d,w,(1<<2)|(1<<3)|(1<<6))
x.XMapWindow.argtypes=[P,U];x.XMapWindow(d,w)
x.XInternAtom.argtypes=[P,C.c_char_p,I];x.XInternAtom.restype=U
x.XChangeProperty.argtypes=[P,U,U,U,I,I,P,I]
a=U(w);pid=U(os.getpid())
x.XChangeProperty(d,root,x.XInternAtom(d,b'_NET_ACTIVE_WINDOW',0),33,32,0,C.byref(a),1)
x.XChangeProperty(d,w,x.XInternAtom(d,b'_NET_WM_PID',0),6,32,0,C.byref(pid),1)
x.XSync.argtypes=[P,I];x.XSync(d,0)
with X11(w,os.getpid()) as target:
    xi=XI2(target,{w});rec=XIRecorder(target,xi.opcode,xi.devices,{w});xi.action=rec.action=1
    target.move((.5,.5));target.settle_pointer();target.button(True)
    for _ in range(5):xi.poll();rec.poll();time.sleep(.01)
    target.button(False)
    for _ in range(5):xi.poll();rec.poll();time.sleep(.01)
    assert any(r['kind']=='core_up' for r in rec.records),rec.records
    assert any(r['kind']=='raw_button_up' for r in xi.records),xi.records
    assert all(r['pointer']['scope_target']==w for r in xi.records)
    assert all(r['kind'] in ('raw_motion','raw_button_down','raw_button_up') for r in xi.records)
    assert all(not r['pointer']['buttons'] or not r['pointer']['buttons'][0]&2 for r in xi.records if r['kind']=='raw_button_up')
    rec.close();xi.close();assert xi.closed
    assert target.pointer()['mask']&0x100==0
x.XDestroyWindow.argtypes=[P,U];x.XDestroyWindow(d,w);x.XCloseDisplay.argtypes=[P];x.XCloseDisplay(d)
print('UIO3 Xvfb: exact raw source, core delivery, post-release button state, closed observer; physical touch is not claimed')

"""Actual X server mechanics; run with xvfb-run, never a user desktop."""
import ctypes as C
import ctypes.util
import os
import time
from x11 import X11,U,I,P,summaries
from xrecord import Recorder

assert os.environ.get('UIO1_XVFB_FIXTURE')=='1','explicit isolated Xvfb fixture required'
x=C.CDLL(ctypes.util.find_library('X11'))
x.XOpenDisplay.argtypes=[C.c_char_p];x.XOpenDisplay.restype=P;d=x.XOpenDisplay(None);assert d
x.XDefaultRootWindow.argtypes=[P];x.XDefaultRootWindow.restype=U;root=x.XDefaultRootWindow(d)
x.XCreateSimpleWindow.argtypes=[P,U,I,I,C.c_uint,C.c_uint,C.c_uint,U,U];x.XCreateSimpleWindow.restype=U
w=x.XCreateSimpleWindow(d,root,100,80,240,160,0,0,0)
x.XSelectInput.argtypes=[P,U,C.c_long];x.XSelectInput(d,w,(1<<2)|(1<<3)|(1<<6))
x.XMapWindow.argtypes=[P,U];x.XMapWindow(d,w)
x.XInternAtom.argtypes=[P,C.c_char_p,I];x.XInternAtom.restype=U
x.XChangeProperty.argtypes=[P,U,U,U,I,I,P,I]
active=U(w);pid=U(os.getpid())
x.XChangeProperty(d,root,x.XInternAtom(d,b'_NET_ACTIVE_WINDOW',0),33,32,0,C.byref(active),1)
x.XChangeProperty(d,w,x.XInternAtom(d,b'_NET_WM_PID',0),6,32,0,C.byref(pid),1)
x.XSync.argtypes=[P,I];x.XSync(d,0)
with X11(w,os.getpid()) as target:
    record=Recorder(target);record.action=1
    m,pixels=target.capture();old,prior=summaries(pixels,m['width'],m['height'],m['stride'])
    target.move((.5,.5));target.button(True);target.move((.75,.5));target.button(False)
    for _ in range(10):record.poll();time.sleep(.01)
    assert len([r for r in record.records if r['kind']==4])==1,record.records
    assert len([r for r in record.records if r['kind']==5])==1,record.records
    assert record.records[0]['target']==w
    target.button(True)
    # The context destructor must release our button, not leave it held.
    record.close()
with X11(w,os.getpid()) as target:assert target.pointer()['mask']&0x100==0
x.XDestroyWindow.argtypes=[P,U];x.XDestroyWindow(d,w);x.XCloseDisplay.argtypes=[P];x.XCloseDisplay(d)
print('UIO1 Xvfb: exact window capture, server-level XTEST, exact-client X RECORD and cancellation passed')

"""Source-owned X server clock calibration. No mouse, touch or keyboard input."""
import ctypes as C
import ctypes.util
import os
from session import xclock,X11,P,U,I
assert os.environ.get('UIR2_X11_FIXTURE')=='1'
lib=C.CDLL(ctypes.util.find_library('X11'))
lib.XOpenDisplay.argtypes=[C.c_char_p];lib.XOpenDisplay.restype=P;d=lib.XOpenDisplay(None);assert d
lib.XDefaultRootWindow.argtypes=[P];lib.XDefaultRootWindow.restype=U;root=lib.XDefaultRootWindow(d)
lib.XCreateSimpleWindow.argtypes=[P,U,I,I,C.c_uint,C.c_uint,C.c_uint,U,U];lib.XCreateSimpleWindow.restype=U
w=lib.XCreateSimpleWindow(d,root,10,10,300,200,0,0,0)
lib.XSync.argtypes=[P,I];lib.XSync(d,0)
try:
 with X11(w) as x:
  a=xclock(x,1);b=xclock(x,2)
  assert a['linux_before_ns']<=a['linux_after_ns']<=b['linux_after_ns']
  from analysis import delta32
  assert 0<=delta32(b['server_ms'],a['server_ms'])<1000
finally:
 lib.XDestroyWindow.argtypes=[P,U];lib.XDestroyWindow(d,w);lib.XCloseDisplay.argtypes=[P];lib.XCloseDisplay(d)
print('UIR2: source-owned PropertyNotify clock, no input, window/connection cleanup passed')

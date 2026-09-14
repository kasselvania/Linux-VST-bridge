"""Isolated real X server popup capture, occlusion and XTEST guard regression."""
import ctypes as C
import os
from pathlib import Path
import sys
import time
sys.path.insert(0,str(Path(__file__).resolve().parent))
from desktop import PopupX11
from popup import Editor
from x11 import X11,I,U,P
from xrecord import Recorder
from dataclasses import asdict
assert os.environ.get('UIO2_XVFB_FIXTURE')=='1','private Xvfb only'
# Bootstrap solely to acquire the existing typed X11 connection.
import ctypes.util
lib=C.CDLL(ctypes.util.find_library('X11'));lib.XOpenDisplay.argtypes=[C.c_char_p];lib.XOpenDisplay.restype=P
bootstrap=lib.XOpenDisplay(None);assert bootstrap
lib.XDefaultRootWindow.argtypes=[P];lib.XDefaultRootWindow.restype=U;root=lib.XDefaultRootWindow(bootstrap)
lib.XCreateSimpleWindow.argtypes=[P,U,I,I,C.c_uint,C.c_uint,C.c_uint,U,U];lib.XCreateSimpleWindow.restype=U
editor=lib.XCreateSimpleWindow(bootstrap,root,40,40,600,400,0,0,0x334455)
popup=lib.XCreateSimpleWindow(bootstrap,root,80,80,240,80,0,0,0x8899aa)
foreign=lib.XCreateSimpleWindow(bootstrap,root,180,90,150,60,0,0,0xcc0000)
lib.XMapWindow.argtypes=[P,U];lib.XUnmapWindow.argtypes=[P,U];lib.XSync.argtypes=[P,I]
lib.XSelectInput.argtypes=[P,U,C.c_long];lib.XSelectInput(bootstrap,popup,(1<<2)|(1<<3)|(1<<6))
for w in (editor,popup):lib.XMapWindow(bootstrap,w)
lib.XInternAtom.argtypes=[P,C.c_char_p,I];lib.XInternAtom.restype=U
lib.XChangeProperty.argtypes=[P,U,U,U,I,I,P,I]
active=U(editor);lib.XChangeProperty(bootstrap,root,lib.XInternAtom(bootstrap,b'_NET_ACTIVE_WINDOW',0),33,32,0,C.byref(active),1);lib.XSync(bootstrap,0)
e=Editor('1'*32,1,1,1,1,5,6,7,8,editor)
def row(hwnd,xid,rect,owner):return dict(hwnd=hwnd,parent=root,owner=owner,root=hwnd,root_owner=8,xid=xid,pid=5,tid=7,visible=1,enabled=1,minimized=0,dpi=96,style=0,reserved=0,rect=rect,client=rect,work=[0,0,1280,800])
r=row(9,popup,[80,80,320,160],8);g=dict(editor=asdict(e),windows=[row(8,editor,[40,40,640,440],0),r])
with PopupX11(r,e,lambda:g,lambda:e,lambda:False) as x:
    meta,pixels=x.capture();assert (meta['width'],meta['height'])==(240,80) and len(pixels)==80*meta['stride']
    rec=Recorder(x);rec.action=1
    x.move((.5,.5));x.settle_pointer();x.button(True);x.button(False)
    for _ in range(20):rec.poll();time.sleep(.005)
    assert [v['kind'] for v in rec.records if v['kind'] in (4,5)]==[4,5]
    lib.XMapWindow(bootstrap,foreign);lib.XSync(bootstrap,0)
    try:x.capture()
    except RuntimeError as err:assert 'occlusion' in str(err)
    else:raise AssertionError('foreign occlusion allowed')
    lib.XUnmapWindow(bootstrap,foreign);lib.XSync(bootstrap,0)
    assert x.capture()[0]['width']==240
    # Terminal status is checked before any input or frame read.
    x.terminal=lambda:True
    try:x.capture()
    except RuntimeError as err:assert 'terminal' in str(err)
    else:raise AssertionError('terminal interaction allowed')
    x.terminal=lambda:False
    # Redirect only this source-owned isolated fixture, never a vendor window.
    x.c.XCompositeRedirectWindow.argtypes=[P,U,I]
    x.c.XCompositeUnredirectWindow.argtypes=[P,U,I]
    x.c.XCompositeRedirectWindow(x.display,popup,0);x.sync();x.capture_backend=None
    redirected,raw=x.capture();assert redirected['capture_backend']=='xcomposite_pixmap'
    assert redirected['effective_masks']==(0xff0000,0xff00,0xff)
    x.c.XCompositeUnredirectWindow(x.display,popup,0);x.sync()
    rec.close()
lib.XDestroyWindow.argtypes=[P,U];lib.XCloseDisplay.argtypes=[P]
# Real BadWindow between a source-owned Win32-shaped snapshot and X11 query.
# The collector/finalizer is the production SurfaceGraphX11 owner. No input is
# issued by the census path; the deleted popup can never reach the action binder.
from desktop import SurfaceGraphX11
from popup import bind
with SurfaceGraphX11(editor) as census:
    transient=lib.XCreateSimpleWindow(bootstrap,root,90,90,100,60,0,0,0x445566)
    lib.XMapWindow(bootstrap,transient);lib.XSync(bootstrap,0)
    stale=row(10,transient,[90,90,190,150],8)
    attempts=[]
    def snapshot():
        rows=[g['windows'][0]]
        if not attempts:
            rows.append(stale)
            lib.XDestroyWindow(bootstrap,transient);lib.XSync(bootstrap,0)
        attempts.append(len(rows))
        return dict(editor=asdict(e),windows=rows,interval_ns=[time.monotonic_ns(),time.monotonic_ns()])
    result=census.snapshot(snapshot)
    assert attempts==[2,1] and result['discarded_attempts']==1
    assert str(transient) not in result['x11'] and not census.buttons and not census.keys
    try:bind(result,e,e)
    except RuntimeError:pass
    else:raise AssertionError('deleted transient authorized')
    # A fresh Win32 census that keeps claiming a missing visible XID is refused.
    def unresolved():
        return dict(editor=asdict(e),windows=[g['windows'][0],stale],interval_ns=[time.monotonic_ns(),time.monotonic_ns()])
    try:census.snapshot(unresolved)
    except RuntimeError as err:assert '3 complete attempts' in str(err)
    else:raise AssertionError('unverifiable visible surface admitted')
    print('UIO2 real BadWindow: bounded fresh census, no stale popup authority or input passed')
for w in (popup,foreign,editor):lib.XDestroyWindow(bootstrap,w)
lib.XCloseDisplay(bootstrap)
print('UIO2 Xvfb: popup capture, normal input receipt, foreign occlusion and terminal refusal passed')

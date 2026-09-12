"""Bounded exact-window X11/XWayland edge. No shell input synthesis.

XTEST is server input, not a promise of delivery to a particular client. Every
action checks current geometry, pointer/active window and then uses the Windows
observer as receipt. This module never focuses a Mac application or uses XSendEvent.
Native Wayland windows require the separate consent/portal boundary.
"""
import array
import ctypes as C
import ctypes.util
import hashlib
import math
import struct
import time

U=C.c_ulong; I=C.c_int; P=C.c_void_p

class Image(C.Structure):
    _fields_=[('width',I),('height',I),('xoffset',I),('format',I),('data',P),
              ('byte_order',I),('bitmap_unit',I),('bitmap_bit_order',I),('bitmap_pad',I),
              ('depth',I),('bytes_per_line',I),('bits_per_pixel',I),
              ('red_mask',U),('green_mask',U),('blue_mask',U),('obdata',P),('functions',P*6)]

def normalized(rect, point):
    x,y,w,h=rect
    if w<1 or h<1 or any(not math.isfinite(v) or not 0<=v<=1 for v in point):
        raise ValueError('invalid normalized target')
    return x+round(point[0]*(w-1)),y+round(point[1]*(h-1))

def summaries(pixels, width, height, stride, previous=None, regions=()):
    """8-pixel spatial sample, not a full-image pixel-change percentage.
    Hashes of explicit regions are exact. Animated areas can be omitted by the
    caller; signature equality alone is never semantic page identification.
    """
    if width<1 or height<1 or width>4096 or height>4096 or stride<width*4 or len(pixels)!=stride*height:
        raise ValueError('frame extent')
    words=array.array('I');words.frombytes(pixels)
    sampled=[]
    for y in range(0,height,8):sampled.extend(words[y*(stride//4):y*(stride//4)+width:8])
    nw=(width+7)//8
    changed=[] if previous is None else [i for i,(a,b) in enumerate(zip(sampled,previous)) if (a&0xffffff)!=(b&0xffffff)]
    if previous is not None and len(previous)!=len(sampled):raise ValueError('geometry changed')
    box=None
    if changed:
        xs=[(i%nw)*8 for i in changed];ys=[(i//nw)*8 for i in changed]
        box=[min(xs),min(ys),min(width,max(xs)+8),min(height,max(ys)+8)]
    shades=[sum((sampled[min(len(sampled)-1,i*len(sampled)//64)]>>b)&255 for b in (0,8,16)) for i in range(64)]
    median=sorted(shades)[32];perceptual=sum((v>=median)<<i for i,v in enumerate(shades))
    hashes=[]
    for r in regions:
        x,y,w,h=r
        if min(x,y)<0 or min(w,h)<1 or x+w>width or y+h>height:raise ValueError('region outside window')
        digest=hashlib.sha256()
        for row in range(y,y+h):digest.update(pixels[row*stride+x*4:row*stride+(x+w)*4])
        hashes.append(digest.hexdigest())
    return dict(sha256=hashlib.sha256(pixels).hexdigest(),perceptual_hash=f'{perceptual:016x}',
                changed_sample_percent=100*len(changed)/len(sampled),changed_sample_bounds=box,
                spatial_step=8,region_hashes=hashes),sampled

class X11:
    def __init__(self, window, pid=None):
        self.x=C.CDLL(ctypes.util.find_library('X11'));self.t=C.CDLL(ctypes.util.find_library('Xtst'))
        self.c=C.CDLL(ctypes.util.find_library('Xcomposite'));self.window=window;self.pid=pid
        def bind(lib,name,result,args):
            f=getattr(lib,name);f.restype=result;f.argtypes=args;return f
        bind(self.x,'XOpenDisplay',P,[C.c_char_p]);bind(self.x,'XCloseDisplay',I,[P]);bind(self.x,'XDefaultRootWindow',U,[P]);
        bind(self.x,'XGetGeometry',I,[P,U,C.POINTER(U),C.POINTER(I),C.POINTER(I),C.POINTER(C.c_uint),C.POINTER(C.c_uint),C.POINTER(C.c_uint),C.POINTER(C.c_uint)])
        bind(self.x,'XTranslateCoordinates',I,[P,U,U,I,I,C.POINTER(I),C.POINTER(I),C.POINTER(U)])
        bind(self.x,'XInternAtom',U,[P,C.c_char_p,I]);bind(self.x,'XGetWindowProperty',I,[P,U,U,C.c_long,C.c_long,I,U,C.POINTER(U),C.POINTER(I),C.POINTER(U),C.POINTER(U),C.POINTER(P)])
        bind(self.x,'XFree',I,[P]);bind(self.x,'XSync',I,[P,I]);bind(self.x,'XQueryPointer',I,[P,U,C.POINTER(U),C.POINTER(U),C.POINTER(I),C.POINTER(I),C.POINTER(I),C.POINTER(I),C.POINTER(C.c_uint)])
        bind(self.x,'XGetInputFocus',I,[P,C.POINTER(U),C.POINTER(I)]);bind(self.x,'XGetImage',C.POINTER(Image),[P,U,I,I,C.c_uint,C.c_uint,U,I]);bind(self.x,'XDestroyImage',I,[C.POINTER(Image)]);bind(self.x,'XFreePixmap',I,[P,U])
        bind(self.t,'XTestQueryExtension',I,[P,C.POINTER(I),C.POINTER(I),C.POINTER(I),C.POINTER(I)])
        bind(self.t,'XTestFakeMotionEvent',I,[P,I,I,I,U]);bind(self.t,'XTestFakeButtonEvent',I,[P,C.c_uint,I,U]);bind(self.t,'XTestFakeKeyEvent',I,[P,C.c_uint,I,U])
        bind(self.c,'XCompositeQueryExtension',I,[P,C.POINTER(I),C.POINTER(I)]);bind(self.c,'XCompositeNameWindowPixmap',U,[P,U])
        self.capture_backend=None
        self.display=self.x.XOpenDisplay(None)
        if not self.display:raise RuntimeError('X11 display unavailable; native Wayland needs portal consent')
        self.root=self.x.XDefaultRootWindow(self.display);self.buttons=set();self.keys=set();self.pixmap=0;self.errors=[]
        # Our private X connection only; never suppress the application error handler.
        class Error(C.Structure):_fields_=[('type',I),('display',P),('resource',U),('serial',U),('code',C.c_ubyte),('major',C.c_ubyte),('minor',C.c_ubyte)]
        self.error_callback=C.CFUNCTYPE(I,P,C.POINTER(Error))(lambda d,e:self.errors.append((e.contents.code,e.contents.major,e.contents.minor)) or 0)
        self.x.XSetErrorHandler.argtypes=[P];self.x.XSetErrorHandler.restype=P;self.prior_handler=self.x.XSetErrorHandler(self.error_callback)
        a,b,c,d=I(),I(),I(),I()
        if not self.t.XTestQueryExtension(self.display,C.byref(a),C.byref(b),C.byref(c),C.byref(d)):raise RuntimeError('XTEST unavailable')
        self.initial=self.geometry();self.check_identity()
    def sync(self):
        self.x.XSync(self.display,0)
        if self.errors:
            errors=self.errors;self.errors=[];raise RuntimeError(f'X11 operation refused: {errors}')
    def property(self, window, name):
        atom=self.x.XInternAtom(self.display,name.encode(),1)
        if not atom:return []
        kind=U();fmt=I();count=U();after=U();data=P()
        result=self.x.XGetWindowProperty(self.display,window,atom,0,32,0,0,C.byref(kind),C.byref(fmt),C.byref(count),C.byref(after),C.byref(data))
        self.sync()
        try:
            if result or fmt.value!=32 or after.value or count.value>32:return []
            return list((U*count.value).from_address(data.value)) if data.value else []
        finally:
            if data.value:self.x.XFree(data)
    def geometry(self):
        root=U();x=I();y=I();w=C.c_uint();h=C.c_uint();border=C.c_uint();depth=C.c_uint();child=U()
        ok=self.x.XGetGeometry(self.display,self.window,C.byref(root),C.byref(x),C.byref(y),C.byref(w),C.byref(h),C.byref(border),C.byref(depth))
        self.sync()
        if not ok or not 1<=w.value<=4096 or not 1<=h.value<=4096:raise RuntimeError('window geometry unavailable')
        if not self.x.XTranslateCoordinates(self.display,self.window,self.root,0,0,C.byref(x),C.byref(y),C.byref(child)):raise RuntimeError('window translation failed')
        self.sync();return x.value,y.value,w.value,h.value
    def check_identity(self):
        if self.pid is not None and self.property(self.window,'_NET_WM_PID')!=[self.pid]:raise RuntimeError('X11 process identity changed')
        return self.geometry()
    def pointer(self):
        root=U();child=U();rx=I();ry=I();wx=I();wy=I();mask=C.c_uint();focus=U();revert=I()
        ok=self.x.XQueryPointer(self.display,self.window,C.byref(root),C.byref(child),C.byref(rx),C.byref(ry),C.byref(wx),C.byref(wy),C.byref(mask))
        self.x.XGetInputFocus(self.display,C.byref(focus),C.byref(revert));self.sync()
        return dict(same_screen=bool(ok),screen=[rx.value,ry.value],client=[wx.value,wy.value],mask=mask.value,focus=focus.value,child=child.value,active=self.property(self.root,'_NET_ACTIVE_WINDOW'))
    def move(self, point):
        rect=self.check_identity();x,y=normalized(rect,point);before=time.monotonic_ns()
        if not self.t.XTestFakeMotionEvent(self.display,-1,x,y,0):raise RuntimeError('XTEST motion refused')
        self.sync();return dict(kind='motion',interval_ns=[before,time.monotonic_ns()],target=[x,y],pointer=self.pointer())
    def button(self, down, button=1):
        if button not in (1,3):raise ValueError('only bounded primary/secondary buttons')
        if down:
            rect=self.check_identity();p=self.pointer()
            if not 0<=p['client'][0]<rect[2] or not 0<=p['client'][1]<rect[3] or p['active']!=[self.window]:raise RuntimeError('target not active under pointer')
            if p['mask']&0x1fff:raise RuntimeError('operator key/button held; refuse shared input')
            self.buttons.add(button) # retain ownership before sending, for failure cleanup
        elif button not in self.buttons:raise RuntimeError('not our held button')
        before=time.monotonic_ns()
        if not self.t.XTestFakeButtonEvent(self.display,button,int(down),0):raise RuntimeError('XTEST button refused')
        self.sync()
        if not down:self.buttons.remove(button)
        return dict(kind='button_down' if down else 'button_up',interval_ns=[before,time.monotonic_ns()],pointer=self.pointer())
    def key(self, down, name):
        # A closed non-text vocabulary. Keycodes are resolved on this server.
        symbols={'space':0x20,'escape':0xff1b,'left':0xff51,'right':0xff53}
        if name not in symbols:raise ValueError('unsupported diagnostic key')
        self.x.XKeysymToKeycode.argtypes=[P,U];self.x.XKeysymToKeycode.restype=C.c_ubyte
        keycode=self.x.XKeysymToKeycode(self.display,symbols[name])
        if not 8<=keycode<=255:raise RuntimeError('key unavailable')
        if down:
            self.check_identity();p=self.pointer()
            if p['mask']&0x1fff or p['active']!=[self.window]:raise RuntimeError('shared input/focus changed')
            self.keys.add(keycode)
        elif keycode not in self.keys:raise RuntimeError('not our key')
        if not self.t.XTestFakeKeyEvent(self.display,keycode,int(down),0):raise RuntimeError('XTEST key refused')
        self.sync()
        if not down:self.keys.remove(keycode)
    def activate(self):
        # Normal application-origin EWMH request, not synthetic pointer/key
        # input or forced XSetInputFocus. Refusal is a reported outcome.
        class Client(C.Structure):
            _fields_=[('type',I),('serial',U),('send_event',I),('display',P),('window',U),('message',U),('format',I),('data',C.c_long*5)]
        class Event(C.Union):_fields_=[('client',Client),('pad',C.c_long*24)]
        self.check_identity();e=Event();e.client.type=33;e.client.display=self.display;e.client.window=self.window
        e.client.message=self.x.XInternAtom(self.display,b'_NET_ACTIVE_WINDOW',0);e.client.format=32;e.client.data[0]=1
        self.x.XSendEvent.argtypes=[P,U,I,C.c_long,C.POINTER(Event)]
        before=time.monotonic_ns()
        if not self.x.XSendEvent(self.display,self.root,0,(1<<19)|(1<<20),C.byref(e)):raise RuntimeError('activation request refused')
        self.sync();deadline=time.monotonic()+.75
        while self.property(self.root,'_NET_ACTIVE_WINDOW')!=[self.window]:
            if time.monotonic()>deadline:raise RuntimeError('window manager refused activation')
            time.sleep(.01)
        return dict(kind='activation',interval_ns=[before,time.monotonic_ns()])
    def capture(self):
        rect=self.check_identity()
        if rect[2:]!=self.initial[2:]:
            if self.pixmap:self.x.XFreePixmap(self.display,self.pixmap)
            self.pixmap=0;self.initial=rect
        if not self.pixmap and self.capture_backend!='direct_drawable':
            a,b=I(),I()
            if not self.c.XCompositeQueryExtension(self.display,C.byref(a),C.byref(b)):raise RuntimeError('XComposite unavailable')
            self.pixmap=self.c.XCompositeNameWindowPixmap(self.display,self.window)
            self.x.XSync(self.display,0)
            if self.errors==[(8,142,6)] or (len(self.errors)==1 and self.errors[0][0]==8 and self.errors[0][2]==6):
                # XWayland may have no XComposite redirection. Do not redirect
                # it (that would change the renderer). Read only its own drawable
                # while foreground; occluded/minimized frames are not evidence.
                self.errors=[];self.pixmap=0;self.capture_backend='direct_drawable'
            else:
                self.sync();self.capture_backend='xcomposite_pixmap'
                if not self.pixmap:raise RuntimeError('exact pixmap unavailable')
        if self.capture_backend=='direct_drawable' and self.property(self.root,'_NET_ACTIVE_WINDOW')!=[self.window]:
            raise RuntimeError('direct drawable is not foreground; occlusion not qualified')
        before=time.monotonic_ns();p=self.x.XGetImage(self.display,self.pixmap or self.window,0,0,rect[2],rect[3],U(-1).value,2)
        self.sync()
        if not p:raise RuntimeError('exact pixmap capture failed')
        try:
            a=p.contents
            if a.bits_per_pixel!=32 or a.byte_order!=0 or (a.red_mask,a.green_mask,a.blue_mask)!=(0xff0000,0xff00,0xff) or a.bytes_per_line>4096*4:raise RuntimeError('unsupported pixel format')
            pixels=C.string_at(a.data,a.bytes_per_line*a.height)
            return dict(interval_ns=[before,time.monotonic_ns()],width=a.width,height=a.height,stride=a.bytes_per_line,capture_backend=self.capture_backend),pixels
        finally:self.x.XDestroyImage(p)
    def close(self):
        # Never release a key/button we did not press. Up remains necessary even
        # if the target has disappeared or focus changed after Down.
        for b in self.buttons:self.t.XTestFakeButtonEvent(self.display,b,0,0)
        for k in self.keys:self.t.XTestFakeKeyEvent(self.display,k,0,0)
        self.buttons.clear();self.keys.clear();self.x.XSync(self.display,0)
        if self.pixmap:self.x.XFreePixmap(self.display,self.pixmap)
        self.x.XCloseDisplay(self.display);self.x.XSetErrorHandler(self.prior_handler)
    def __enter__(self):return self
    def __exit__(self,*args):self.close()

"""Read-only XI2 pointer/touch edge. No touch selection/grab on vendor windows.

Raw events are selected on this observer's root connection only, excluding every
keyboard class. Raw events have NO target: their target is a later queried sample,
never represented as delivery. Actual targeted XI2 delivery comes from X RECORD
on the exact vendor client. Touch ownership, pointer emulation and grabs are not
changed. XWayland may expose only a virtual pointer, not physical touch events.
"""
import ctypes as C
import ctypes.util
import hashlib
import struct
import time
from x11 import P,I,U,_attach_errors,_detach_errors
from xrecord import Recorder,Range8,pointer_packet

DEVICE_EVENTS={4:'xi_button_down',5:'xi_button_up',6:'xi_motion',18:'xi_touch_begin',19:'xi_touch_update',20:'xi_touch_end'}
RAW_EVENTS={15:'raw_button_down',16:'raw_button_up',17:'raw_motion',22:'raw_touch_begin',23:'raw_touch_update',24:'raw_touch_end'}


def packets(raw,swapped=False):
    """XGE length excludes first 32 bytes. Never step generic events by 32."""
    endian='>' if swapped else '<';at=0
    while at<len(raw):
        if len(raw)-at<32:raise ValueError('truncated X event')
        size=32
        if raw[at]&127==35:size+=4*struct.unpack_from(endian+'I',raw,at+4)[0]
        if size>8192 or at+size>len(raw):raise ValueError('X event extent')
        yield raw[at:at+size];at+=size


def device_packet(b,opcode,targets,devices,swapped=False):
    if len(b)<32 or b[0]!=35 or b[1]!=opcode:return None
    e='>' if swapped else '<';kind,device=struct.unpack_from(e+'HH',b,8)
    if kind not in DEVICE_EVENTS:return None # never read keyboard detail
    if len(b)<80 or len(b)!=32+4*struct.unpack_from(e+'I',b,4)[0]:raise ValueError('XI2 extent')
    stamp,detail,root,target,child=struct.unpack_from(e+'5I',b,12)
    buttons,valuators,source=struct.unpack_from(e+'3H',b,48)
    if target not in targets or source not in devices or device not in devices:return None
    mask_end=80+4*(buttons+valuators)
    if buttons>8 or valuators>8 or mask_end>len(b):raise ValueError('XI2 mask bound')
    axis_count=sum(v.bit_count() for v in b[80+4*buttons:mask_end])
    if mask_end+axis_count*8!=len(b):raise ValueError('XI2 valuator extent')
    rx,ry,x,y=struct.unpack_from(e+'4i',b,32)
    return dict(kind=DEVICE_EVENTS[kind],device=device,source=source,detail=detail,target=target,child=child,
                server_ms=stamp,screen=[rx/65536,ry/65536],client=[x/65536,y/65536],
                flags=struct.unpack_from(e+'I',b,56)[0],buttons=list(b[80:80+buttons*4]),target_basis='delivered')


class XIRecorder(Recorder):
    def __init__(self,x,opcode,devices,targets):
        self.opcode=opcode;self.devices=devices;self.targets=set(targets)
        # Recorder provides exact client custody; widen only its event mask.
        super().__init__(x)
    def ranges(self):
        from xrecord import Range
        core=Range();core.delivered=Range8(4,6)
        generic=Range();generic.delivered=Range8(35,35)
        return [core,generic]
    def receive(self,closure,ptr):
        d=ptr.contents
        try:
            if d.category!=0 or not self.action:return
            if d.length>8192:self.dropped+=1;return
            raw=C.string_at(d.data,d.length*4)
            try:
                for b in packets(raw,bool(d.swapped)):
                    row=None
                    if b[0]&127 in (4,5,6):
                        for target in self.targets:
                            row=pointer_packet(b,target,bool(d.swapped))
                            if row:break
                        if row:row['kind']={4:'core_down',5:'core_up',6:'core_motion'}[row['kind']]
                    elif b[0]==35:row=device_packet(b,self.opcode,self.targets,self.devices,bool(d.swapped))
                    if row is None:self.unparsed+=1;continue
                    if len(self.records)>=self.capacity:self.dropped+=1;continue
                    self.records.append(dict(row,action=self.action,observed_ns=time.monotonic_ns()))
            except ValueError:self.dropped+=1
        finally:self.x.t.XRecordFreeData(ptr)


class Mask(C.Structure):_fields_=[('device',I),('length',I),('mask',C.POINTER(C.c_ubyte))]
class AnyClass(C.Structure):_fields_=[('type',I),('source',I)]
class TouchClass(C.Structure):_fields_=[('type',I),('source',I),('mode',I),('count',I)]
class Device(C.Structure):
    _fields_=[('id',I),('name',C.c_char_p),('use',I),('attachment',I),('enabled',I),('count',I),('classes',C.POINTER(C.POINTER(AnyClass)))]
class Valuators(C.Structure):_fields_=[('length',I),('mask',P),('values',P)]
class Raw(C.Structure):
    _fields_=[('type',I),('serial',U),('send',I),('display',P),('extension',I),('evtype',I),('time',U),('device',I),('source',I),('detail',I),('flags',I),('valuators',Valuators),('raw_values',P)]
class Cookie(C.Structure):
    _fields_=[('type',I),('serial',U),('send',I),('display',P),('extension',I),('evtype',I),('cookie',C.c_uint),('data',P)]
class Event(C.Union):_fields_=[('cookie',Cookie),('pad',C.c_long*24)]
class Buttons(C.Structure):_fields_=[('length',I),('mask',C.POINTER(C.c_ubyte))]
class Mods(C.Structure):_fields_=[('base',I),('latched',I),('locked',I),('effective',I)]


class RawScope:
    """Raw touch origin binds once to an exact queried target, source and touch ID.
    A continued contact may leave that target; only the admitted contact persists.
    Device generation is checked separately. No unrelated raw input is retained.
    """
    def __init__(self,targets,devices):self.targets=set(targets);self.devices=set(devices);self.contacts=set()
    def admit(self,kind,source,detail,target):
        if source not in self.devices:return False
        key=(source,detail)
        if kind=='raw_touch_begin':
            if target not in self.targets or len(self.contacts)>=16:return False
            self.contacts.add(key);return True
        if kind in ('raw_touch_update','raw_touch_end'):
            if key not in self.contacts:return False
            if kind=='raw_touch_end':self.contacts.remove(key)
            return True
        return target in self.targets


class XI2:
    def __init__(self,x,targets,capacity=4096):
        self.x=x;self.display=x.x.XOpenDisplay(None);self.errors=[];self.records=[];self.dropped=0;self.capacity=capacity;self.action=0;self.closed=False
        if not self.display:raise RuntimeError('XI2 private display')
        _attach_errors(x.x,self.display,self.errors)
        try:
            self.lib=C.CDLL(ctypes.util.find_library('Xi'))
            def bind(lib,name,result,args):f=getattr(lib,name);f.restype=result;f.argtypes=args
            bind(x.x,'XQueryExtension',I,[P,C.c_char_p,C.POINTER(I),C.POINTER(I),C.POINTER(I)])
            bind(x.x,'XPending',I,[P]);bind(x.x,'XNextEvent',I,[P,C.POINTER(Event)])
            bind(x.x,'XGetEventData',I,[P,C.POINTER(Cookie)]);bind(x.x,'XFreeEventData',None,[P,C.POINTER(Cookie)])
            bind(self.lib,'XIQueryVersion',I,[P,C.POINTER(I),C.POINTER(I)])
            bind(self.lib,'XIQueryDevice',C.POINTER(Device),[P,I,C.POINTER(I)]);bind(self.lib,'XIFreeDeviceInfo',None,[C.POINTER(Device)])
            bind(self.lib,'XISelectEvents',I,[P,U,C.POINTER(Mask),I])
            bind(self.lib,'XIQueryPointer',I,[P,I,U,C.POINTER(U),C.POINTER(U),*[C.POINTER(C.c_double)]*4,C.POINTER(Buttons),C.POINTER(Mods),C.POINTER(Mods)])
            op,a,b=I(),I(),I()
            if not x.x.XQueryExtension(self.display,b'XInputExtension',C.byref(op),C.byref(a),C.byref(b)):raise RuntimeError('XI2 unavailable')
            self.opcode=op.value;major,minor=I(2),I(2)
            if self.lib.XIQueryVersion(self.display,C.byref(major),C.byref(minor))!=0 or (major.value,minor.value)<(2,2):raise RuntimeError('XI2.2 required')
            self.version=[major.value,minor.value];self.devices=self.census();self.scope=RawScope(targets,self.devices)
            bits=(C.c_ubyte*4)()
            for kind in RAW_EVENTS:bits[kind//8]|=1<<(kind%8)
            masks=(Mask*len(self.devices))(*(Mask(d,4,bits) for d,v in self.devices.items()))
            # Per-device raw selection cannot claim or reject a touch sequence.
            if self.lib.XISelectEvents(self.display,x.root,masks,len(masks))!=0:raise RuntimeError('raw selection refused')
            x.x.XSync(self.display,False)
            if self.errors:raise RuntimeError('XI2 selection error')
        except BaseException:self.close();raise
    def census(self):
        count=I();ptr=self.lib.XIQueryDevice(self.display,0,C.byref(count));rows={}
        if not ptr:raise RuntimeError('XI2 device census unavailable')
        try:
            if not 0<count.value<=64:raise RuntimeError('XI2 device capacity')
            for i in range(count.value):
                d=ptr[i]
                if d.use not in (1,3,5) or not d.enabled:continue # exclude keyboards entirely
                if not 0<=d.count<=32:raise RuntimeError('XI2 class capacity')
                kinds=[];touch=None
                for j in range(d.count):
                    cls=d.classes[j].contents;kinds.append(cls.type)
                    if cls.type==8:
                        t=C.cast(d.classes[j],C.POINTER(TouchClass)).contents;touch=dict(mode=t.mode,max_contacts=t.count)
                name=d.name or b''
                if len(name)>256:raise RuntimeError('XI2 device name capacity')
                rows[d.id]=dict(use=d.use,attachment=d.attachment,classes=sorted(kinds),touch=touch,name_sha256=hashlib.sha256(name).hexdigest())
            if not rows:raise RuntimeError('XI2 pointer devices unavailable')
            return rows
        finally:self.lib.XIFreeDeviceInfo(ptr)
    def pointer(self,source):
        d=self.devices[source];master=source if d['use']==1 else d['attachment']
        if master not in self.devices:return dict(available=False,target=None)
        root,child=U(),U();rx,ry,x,y=(C.c_double() for _ in range(4));buttons=Buttons();mods,group=Mods(),Mods()
        before=time.monotonic_ns();chain=[];window=self.x.root;mask_bytes=[]
        # Walk the server's actual pointer chain using the exact XI master. A
        # WM frame may precede the editor; do not equate root child with editor.
        for _ in range(16):
            buttons=Buttons()
            ok=self.lib.XIQueryPointer(self.display,master,window,C.byref(root),C.byref(child),C.byref(rx),C.byref(ry),C.byref(x),C.byref(y),C.byref(buttons),C.byref(mods),C.byref(group))
            try:
                if not ok or not 0<=buttons.length<=32:return dict(available=False,target=None)
                mask_bytes=list(C.string_at(buttons.mask,buttons.length)) if buttons.mask else []
            finally:
                if buttons.mask:self.x.x.XFree(buttons.mask)
            chain.append(int(window))
            if not child.value:break
            window=int(child.value)
        else:raise RuntimeError('XI pointer ancestry bound')
        return dict(available=True,target=window,scope_target=self.x.window if self.x.window in chain else None,
                    chain=chain,master=master,screen=[rx.value,ry.value],buttons=mask_bytes,
                    observed_interval_ns=[before,time.monotonic_ns()],target_basis='query_after_raw_not_event_delivery')
    def poll(self):
        for _ in range(256):
            if not self.x.x.XPending(self.display):break
            e=Event();self.x.x.XNextEvent(self.display,C.byref(e));c=e.cookie
            if c.type!=35 or c.extension!=self.opcode or c.evtype not in RAW_EVENTS:continue
            if not self.x.x.XGetEventData(self.display,C.byref(c)):self.dropped+=1;continue
            try:
                if not self.action:continue
                r=C.cast(c.data,C.POINTER(Raw)).contents
                if r.send or r.source not in self.devices or r.device not in self.devices:continue
                point=self.pointer(r.source);kind=RAW_EVENTS[r.evtype]
                if not self.scope.admit(kind,r.source,r.detail,point.get('scope_target')):continue
                if len(self.records)>=self.capacity:self.dropped+=1;continue
                self.records.append(dict(kind=kind,device=r.device,source=r.source,detail=r.detail,flags=r.flags,server_ms=r.time,action=self.action,observed_ns=time.monotonic_ns(),pointer=point))
            finally:self.x.x.XFreeEventData(self.display,C.byref(c))
        if self.errors:raise RuntimeError('XI2 asynchronous error')
    def close(self):
        if self.closed:return
        self.closed=True
        if self.display:_detach_errors(self.x.x,self.display);self.x.x.XCloseDisplay(self.display);self.display=None

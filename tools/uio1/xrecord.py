"""Read-only X RECORD for the client owning one exact XID, never all clients.
Only pointer-event scalars addressed to that XID are retained. No keyboard bytes,
client requests, property contents or protocol payloads are retained.
"""
import ctypes as C
import struct
import time
from x11 import U,I,P
class Range8(C.Structure):_fields_=[('first',C.c_ubyte),('last',C.c_ubyte)]
class Range16(C.Structure):_fields_=[('first',C.c_ushort),('last',C.c_ushort)]
class ExtRange(C.Structure):_fields_=[('major',Range8),('minor',Range16)]
class Range(C.Structure):
    _fields_=[('requests',Range8),('replies',Range8),('ext_requests',ExtRange),('ext_replies',ExtRange),('delivered',Range8),('device',Range8),('errors',Range8),('started',I),('died',I)]
class Data(C.Structure):
    _fields_=[('base',U),('server_time',U),('sequence',U),('category',I),('swapped',I),('data',P),('length',U)]

def pointer_packet(b, window, swapped=False):
    if len(b)<32:return None
    endian='>' if swapped else '<';kind=b[0]&127
    # Core ButtonPress/Release/Motion only. Generic events are explicitly counted
    # as unparsed; they are never guessed to be XI2 pointer events.
    if kind not in (4,5,6):return None
    stamp,root,target,child=struct.unpack_from(endian+'IIII',b,4)
    if target!=window:return None
    rx,ry,x,y,state=struct.unpack_from(endian+'hhhhH',b,20)
    return dict(kind=kind,button=b[1] if kind!=6 else None,server_ms=stamp,
                target=target,child=child,client=[x,y],screen=[rx,ry],state=state)

class Recorder:
    def __init__(self, x, capacity=4096):
        self.x=x;self.records=[];self.dropped=0;self.unparsed=0;self.capacity=capacity;self.action=0
        self.data=x.x.XOpenDisplay(None)
        if not self.data:raise RuntimeError('X RECORD data connection')
        t=x.t
        t.XRecordQueryVersion.argtypes=[P,C.POINTER(I),C.POINTER(I)];t.XRecordQueryVersion.restype=I
        a,b=I(),I()
        if not t.XRecordQueryVersion(x.display,C.byref(a),C.byref(b)):raise RuntimeError('X RECORD unavailable')
        t.XRecordCreateContext.argtypes=[P,I,C.POINTER(U),I,C.POINTER(C.POINTER(Range)),I];t.XRecordCreateContext.restype=U
        t.XRecordDisableContext.argtypes=[P,U];t.XRecordFreeContext.argtypes=[P,U];t.XRecordFreeData.argtypes=[C.POINTER(Data)];t.XRecordProcessReplies.argtypes=[P]
        r=Range();r.delivered=Range8(4,6);rp=C.pointer(r);client=U(x.window)
        self.context=t.XRecordCreateContext(x.display,1,C.byref(client),1,C.byref(rp),1);x.sync()
        if not self.context:raise RuntimeError('exact-client X RECORD context refused')
        self.callback=C.CFUNCTYPE(None,P,C.POINTER(Data))(self.receive)
        t.XRecordEnableContextAsync.argtypes=[P,U,type(self.callback),P];t.XRecordEnableContextAsync.restype=I
        if not t.XRecordEnableContextAsync(self.data,self.context,self.callback,None):raise RuntimeError('X RECORD enable refused')
    def receive(self, closure, ptr):
        d=ptr.contents
        try:
            if d.category!=0 or not self.action:return
            if d.length>4096:self.dropped+=1;return
            raw=C.string_at(d.data,d.length*4)
            for offset in range(0,min(len(raw),4096),32):
                item=pointer_packet(raw[offset:offset+32],self.x.window,bool(d.swapped))
                if item is None:self.unparsed+=1;continue
                if len(self.records)>=self.capacity:self.dropped+=1;continue
                self.records.append(dict(item,action=self.action,observed_ns=time.monotonic_ns()))
        finally:self.x.t.XRecordFreeData(ptr)
    def poll(self):self.x.t.XRecordProcessReplies(self.data)
    def close(self):
        self.x.t.XRecordDisableContext(self.x.display,self.context);self.x.sync();self.poll()
        self.x.t.XRecordFreeContext(self.x.display,self.context);self.x.x.XCloseDisplay(self.data)

"""Scalar custody readers. No consuming or writing the production GUI queues.
All records are private until projected by the Rust UIO1 public report schema.
"""
import ctypes as C
import ctypes.util
import mmap
import os
import pathlib
import stat
import struct
import time

CAPACITY=16384;HEADER=4096;RECORD=112;SIZE=HEADER+CAPACITY*RECORD
NAMES=['commit','qpc','action','hwnd','focus','active','capture','message','source',
       'x','y','screen_x','screen_y','buttons','key_class','result','message_time','cost_ticks']

class Mapping:
    def __init__(self,path,size):
        self.fd=os.open(path,os.O_RDWR|os.O_NOFOLLOW);s=os.fstat(self.fd)
        if not stat.S_ISREG(s.st_mode) or s.st_uid!=os.getuid() or s.st_size!=size or s.st_mode&0o077:
            os.close(self.fd);raise RuntimeError('private status ownership/layout')
        self.map=mmap.mmap(self.fd,size);self.view=(C.c_ubyte*size).from_buffer(self.map)
        self.atomic=C.CDLL(ctypes.util.find_library('atomic'))
        self.load=getattr(self.atomic,'__atomic_load_8');self.load.argtypes=[C.c_void_p,C.c_int];self.load.restype=C.c_uint64
        self.store=getattr(self.atomic,'__atomic_store_8');self.store.argtypes=[C.c_void_p,C.c_uint64,C.c_int]
    def read(self,at):return self.load(C.addressof(self.view)+at,2)
    def write(self,at,value):self.store(C.addressof(self.view)+at,value,3)
    def close(self):del self.view;self.map.close();os.close(self.fd)

class Observer(Mapping):
    def __init__(self,path,pid,start,root):
        super().__init__(path,SIZE)
        if self.map[:4]!=b'UIO1' or struct.unpack_from('<III',self.map,4)!=(1,SIZE,RECORD) or struct.unpack_from('<I',self.map,16)[0]!=pid or self.read(24)!=start or self.read(32)!=root:
            self.close();raise RuntimeError('observer identity/version')
        self.cursor=0;self.clock_serial=0
    def take(self):
        n=self.read(56)
        if n>CAPACITY or n<self.cursor:raise RuntimeError('observer commit invalid')
        rows=[]
        while self.cursor<n:
            at=HEADER+self.cursor*RECORD
            if self.read(at)!=self.cursor+1:break
            r=struct.unpack_from('<7Q2I4i2Iq2Q',self.map,at)
            rows.append(dict(zip(NAMES,r)));self.cursor+=1
        return rows
    def action(self,value):
        if not 0<=value<=4:raise ValueError('action capacity')
        self.write(72,value)
    def clock(self):
        self.clock_serial+=1;before=time.monotonic_ns();self.write(184,self.clock_serial)
        deadline=before+100_000_000
        while self.read(192)!=self.clock_serial:
            if time.monotonic_ns()>deadline:raise RuntimeError('clock bracket deadline')
            time.sleep(.001)
        return dict(linux_before_ns=before,linux_after_ns=time.monotonic_ns(),windows_qpc=self.read(200),frequency=self.read(48))
    def status(self):
        return {name:self.read(offset) for name,offset in dict(committed=56,dropped=64,ready=88,closed=96,
            hook_calls=128,hook_ticks=136,max_hook_ticks=144,filtered=152,heartbeat_errors=160,
            scope_errors=168,unhook_errors=176,detached=216).items()}
    def stop(self):self.write(80,1)

class GuiWitness(Mapping):
    def __init__(self,path,session):
        super().__init__(path,320+2*512*608)
        if self.map[:4]!=b'LVBU' or struct.unpack_from('<I',self.map,4)[0]!=5 or self.map[16:32]!=bytes.fromhex(session):raise RuntimeError('GUI session binding')
        self.cursors=[self.read(64),self.read(80)];self.dropped=[0,0]
    def take(self):
        rows=[]
        for lane,at in enumerate([64,80]):
            before=self.read(at);start=max(self.cursors[lane],before-512)
            self.dropped[lane]+=start-self.cursors[lane]
            for i in range(start,before):
                offset=320+(lane*512+i%512)*608
                # Copy scalar allowlist only: never parameter titles/units.
                a=bytes(self.map[offset:offset+48]);b=bytes(self.map[offset+560:offset+608])
                # A producer may recycle an old slot. Retain only if it was not
                # lapped during this read; this observer is not a queue consumer.
                if self.read(at)>=i+512:self.dropped[lane]+=1;continue
                version,size,kind,parameter,revision,value,flags,steps,count,result=struct.unpack('<4IQd2i2I',a)
                if version!=4 or size!=608:continue
                if kind not in (1,2,3,5,101,102,103,105,106,108):continue
                activation,user_time,requestor,target,epoch,focus,focus_flags,native,lifecycle,reserved=struct.unpack('<Q6IQ2I',b)
                if reserved:continue
                rows.append(dict(lane=lane,queue_sequence=i,observed_ns=time.monotonic_ns(),kind=kind,
                    parameter=parameter,value=value,revision=revision,native_view=native,editor_epoch=epoch,
                    target=target,lifecycle=lifecycle,focus=focus,focus_flags=focus_flags))
            self.cursors[lane]=before
        return rows

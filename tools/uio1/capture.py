"""One bounded three-action editor observation; private artifacts only.

The calibration below is a fixture plan, not a universal UI map or a vendor
runtime policy. Coordinates come from the retained local FX-page image. Input
is XTEST on the Deck, never the Mac. Existing Bitwig automation supplies the
host-driven parameter witness; no production queue is written by this harness.
"""
import ctypes as C
import json
import os
import pathlib
import signal
import sys
import time
import zlib
from launch import Context,private_json
from observe import Observer,GuiWitness,SIZE
from x11 import X11,P,U,I,summaries
from xrecord import Recorder

PROFILE='3126fea7ea72c02bae08fd21cef87e271e5575b09bb419bc1d650ab273e172ad'
REGIONS=[(1080,545,90,90),(800,4,320,36),(25,65,1230,330)]

def class_bytes(x,window):
    atom=x.x.XInternAtom(x.display,b'WM_CLASS',1);kind=U();fmt=I();count=U();after=U();data=P()
    x.x.XGetWindowProperty(x.display,window,atom,0,128,0,0,C.byref(kind),C.byref(fmt),C.byref(count),C.byref(after),C.byref(data));x.sync()
    try:return C.string_at(data,count.value) if data.value and fmt.value==8 and not after.value else b''
    finally:
        if data.value:x.x.XFree(data)

def host_window(x):
    candidates=[]
    for w in x.property(x.root,'_NET_CLIENT_LIST'):
        if class_bytes(x,w) in (b'bitwig-studio\x00BitwigStudio\x00',b'Bitwig Studio\x00Bitwig Studio\x00',b'com.bitwig.BitwigStudio'):
            candidates.append(w)
    if len(candidates)!=1:raise RuntimeError('one exact Bitwig main window required')
    return candidates[0]

def selected_window(rows):
    bindings={r['hwnd']:r['xid'] for r in rows if r['type']=='x11_binding'}
    windows=[r for r in rows if r['type']=='window' and not r['parent'] and r['visible'] and
             not r['minimized'] and r['rect'][2]-r['rect'][0]>320 and bindings.get(r['hwnd'])]
    if len(windows)!=1:raise RuntimeError('ambiguous exact editor root')
    w=windows[0];return w,bindings[w['hwnd']]

def process_sample(pid):
    fields=pathlib.Path(f'/proc/{pid}/stat').read_text().rsplit(')',1)[1].split()
    return dict(at=time.monotonic_ns(),start=int(fields[19]),utime=int(fields[11]),stime=int(fields[12]),hz=os.sysconf('SC_CLK_TCK'))

def renderer(pid):
    names={'d3d9.dll','d3d11.dll','d3d12.dll','dxgi.dll','d2d1.dll','opengl32.dll',
        'winevulkan.dll','wined3d.dll','gdi32.dll','uiautomationcore.dll','libvulkan.so.1','libGL.so.1','libGLX.so.0','libEGL.so.1'}
    found=set();lines=pathlib.Path(f'/proc/{pid}/maps').read_text().splitlines()
    if len(lines)>16384:raise RuntimeError('module mapping bound')
    for line in lines:
        name=line.rsplit('/',1)[-1]
        if name in names:found.add(name)
    return sorted(found)

class Capture:
    def __init__(self,context,out,process,rows,snapshot):
        self.c=context;self.out=out;w,xid=selected_window(rows);self.w=w
        for k in ('DISPLAY','XAUTHORITY'):
            if k in context.env:os.environ[k]=context.env[k]
        self.x=X11(xid);pid=self.x.property(xid,'_NET_WM_PID')
        if len(pid)!=1:raise RuntimeError('exact Linux editor process absent')
        self.x.pid=pid[0];self.x.check_identity();self.pid=pid[0]
        if self.x.geometry()[2:]!=(1280,724):raise RuntimeError('fixture calibration geometry changed')
        self.gui=GuiWitness(pathlib.Path(context.session['directory'])/'ap11.ui',context.session['session'])
        self.before=snapshot;self.action=0;self.frames=[];self.windows=[];self.gestures=[];self.inputs=[];self.brackets=[]
        self.frame_dropped=0;self.previous=None;self.cpu_start=time.process_time_ns();self.stop_requested=False
        self.next_frame=0;self.next_clock=0
        self.result=dict(schema=1,profile_fingerprint=context.admission['profile_fingerprint'],session=context.session['session'],
            process=process,windows=rows,linux_pid=self.pid,renderer=renderer(self.pid),regions=REGIONS,
            process_before=process_sample(self.pid),before=snapshot,frame_capacity=600,frame_interval_ms=100)
        self.helper=None;self.observer=None;self.record=None
    def pump(self,seconds,frames=True):
        deadline=time.monotonic()+seconds
        while time.monotonic()<deadline:
            if self.stop_requested:raise InterruptedError('operator/task cancellation')
            if self.helper:self.helper.poll()
            if self.observer:self.windows.extend(self.observer.take())
            for r in self.gui.take():self.gestures.append(dict(r,action=self.action))
            if self.record:self.record.poll()
            now=time.monotonic()
            if self.observer and now>=self.next_clock:
                self.brackets.append(self.observer.clock());self.next_clock=now+2
            if frames and now>=self.next_frame:
                before=time.process_time_ns();meta,pixels=self.x.capture()
                summary,self.previous=summaries(pixels,meta['width'],meta['height'],meta['stride'],self.previous,REGIONS)
                row=dict(meta,**summary,action=self.action,cpu_ns=time.process_time_ns()-before)
                if len(self.frames)<600:self.frames.append(row)
                else:self.frame_dropped+=1
                self.next_frame=now+.1
            time.sleep(.005)
    def snapshot(self,label):
        if not label or pathlib.Path(label).name!=label or label in ('.','..'):raise ValueError('snapshot label')
        frames=self.out/'snapshots';frames.mkdir(mode=0o700,exist_ok=True)
        meta,pixels=self.x.capture();private_json(frames/(label+'.json'),meta)
        with os.fdopen(os.open(frames/(label+'.bgra.z'),os.O_CREAT|os.O_EXCL|os.O_WRONLY,0o600),'wb') as f:f.write(zlib.compress(pixels,3))
    def mark(self,n):
        self.action=n;self.observer.action(n);self.record.action=n
        self.inputs.append(dict(kind='action_begin',action=n,interval_ns=[time.monotonic_ns()]*2))
    def add(self,row):self.inputs.append(dict(row,action=self.action))
    def run(self,refine=False,manual=False):
        # Baseline is idle overhead context, not an audio-performance result.
        self.snapshot('before');self.pump(2,frames=False)
        self.result['process_trace_off_end']=process_sample(self.pid)
        path=self.out/'ui.bin'
        self.helper=self.c.helper('uio1-observer.exe',['observe',self.w['hwnd'],self.result['process']['start'],90,self.c.runtime.windows(path,self.c.prefix)],self.out/'observer.log',95)
        deadline=time.monotonic()+5
        while not path.exists() or path.stat().st_size!=SIZE:
            self.helper.poll()
            if time.monotonic()>deadline:raise RuntimeError('observer mapping absent')
            time.sleep(.02)
        self.observer=Observer(path,self.result['process']['pid'],self.result['process']['start'],self.w['hwnd'])
        while not self.observer.read(88):
            if time.monotonic()>deadline:raise RuntimeError('observer UI handshake absent')
            self.helper.poll();time.sleep(.02)
        self.record=Recorder(self.x);self.pump(2)
        self.result['process_diagnostic_idle_end']=process_sample(self.pid)
        self.mark(1)
        if manual:
            # Agent/operator/Moonlight input only. The mailbox selects one of
            # four action labels; it cannot supply input, coordinates or code.
            self.result['adapter']='human'
            control=pathlib.Path('/tmp/uio1-manual-action')
            private_json(control,1);self.pump(2);private_json(pathlib.Path('/tmp/uio1-manual-ready'),True)
            end=time.monotonic()+65
            while time.monotonic()<end:
                value=json.loads(control.read_text())
                if value not in (1,2,3,4):raise RuntimeError('manual action label')
                if value!=self.action:
                    self.mark(value)
                    if value==4:self.pump(1);break
                self.pump(.1)
            else:raise TimeoutError('manual observation bound')
            self.snapshot('page');return
        recent=[r for r in self.gestures if r['kind']==3 and r['parameter']==1]
        if not recent and not refine:
            # Only normal Bitwig transport input; the native controller remains
            # the authority that generates parameter updates to the vendor.
            with X11(host_window(self.x)) as daw:
                self.add(daw.activate());daw.key(True,'space');daw.key(False,'space')
                self.add(dict(kind='bitwig_transport_space',interval_ns=[time.monotonic_ns()]*2))
            self.add(self.x.activate())
        self.pump(1 if refine else 6);self.snapshot('host-parameter')
        if not refine and not any(r['action']==1 and r['kind']==3 and r['parameter']==1 for r in self.gestures):
            raise RuntimeError('no host-driven Macro 1 witness; interpret before continuing')
        self.mark(2)
        self.add(self.x.move((1124/1279,586/723)));self.pump(.2);self.add(self.x.settle_pointer())
        self.add(self.x.button(True))
        for y in (580,574,568,562,556):self.add(self.x.move((1124/1279,y/723)));self.pump(.06)
        self.add(self.x.button(False));self.pump(5);self.snapshot('drag')
        self.mark(3)
        self.add(self.x.move((886/1279,22/723)));self.pump(.2);self.add(self.x.settle_pointer())
        self.add(self.x.button(True));self.pump(.09);self.add(self.x.button(False))
        self.pump(6);self.snapshot('page')
        self.mark(4);self.pump(1)
    def close(self):
        try:
            if self.record:self.record.close()
            if self.observer:
                self.observer.action(0);self.observer.stop()
            if self.helper:self.result['helper_cleanup']=self.helper.finish()
            if self.observer:
                self.windows.extend(self.observer.take());self.result['observer_status']=self.observer.status()
                self.result['frequency']=self.observer.read(48);self.observer.close()
        finally:
            try:
                try:process_after=process_sample(self.pid)
                except FileNotFoundError:process_after={'absent':True,'at':time.monotonic_ns()}
                self.result.update(frames=self.frames,win32=self.windows,gui=self.gestures,inputs=self.inputs,brackets=self.brackets,
                    x11=[] if not self.record else self.record.records,x11_dropped=0 if not self.record else self.record.dropped,
                    x11_unparsed=0 if not self.record else self.record.unparsed,gui_dropped=self.gui.dropped,
                    frame_dropped=self.frame_dropped,diagnostic_python_cpu_ns=time.process_time_ns()-self.cpu_start,
                    process_after=process_after,after=self.c.fault.snapshot())
                private_json(self.out/'capture.json',self.result)
            finally:
                self.gui.close();self.x.close();self.c.fault.close()

if __name__=='__main__':
    c=Context(pathlib.Path(sys.argv[1]),sys.argv[2],pathlib.Path(__file__).parent/'package')
    if c.admission['profile_fingerprint']!=PROFILE:raise RuntimeError('fixture plan/profile mismatch')
    mode=sys.argv[3] if len(sys.argv)==4 else 'run'
    if mode not in ('prepare','run','refine','manual'):raise RuntimeError('unknown diagnostic mode')
    out=c.package/('target' if mode=='prepare' else 'interaction-manual' if mode=='manual' else 'interaction-2' if mode=='refine' else 'interaction-1');out.mkdir(mode=0o700)
    process,rows,snapshot=c.census(out/'census.log')
    if mode=='prepare':
        private_json(out/'identity.json',dict(process=process,rows=rows,snapshot=snapshot))
        for k in ('DISPLAY','XAUTHORITY'):
            if k in c.env:os.environ[k]=c.env[k]
        w,xid=selected_window(rows)
        with X11(xid) as x:
            classes=[class_bytes(x,v).decode('ascii','replace') for v in x.property(x.root,'_NET_CLIENT_LIST')]
            print(json.dumps(dict(geometry=x.geometry(),active=x.pointer()['active']==[xid],classes=classes,
                visible_root_count=1,renderer_census_complete=not any(r['type']=='module_census_unavailable' for r in rows))))
        c.fault.close();sys.exit(0)
    capture=Capture(c,out,process,rows,snapshot)
    signal.signal(signal.SIGTERM,lambda *_:setattr(capture,'stop_requested',True))
    signal.signal(signal.SIGINT,lambda *_:setattr(capture,'stop_requested',True))
    try:capture.run(refine=mode=='refine',manual=mode=='manual')
    except BaseException as e:
        capture.result['failure_class']=type(e).__name__;capture.result['failure']=str(e);raise
    finally:capture.close()
    print(json.dumps(dict(actions_complete=True,windows_records=len(capture.windows),gui_records=len(capture.gestures),frames=len(capture.frames),helper_cleanup=capture.result.get('helper_cleanup'))))

"""Bounded candidate-17 observer/capsule owner; no launch, publication or retry.
The custodian establishes Bitwig/project/editor/CA1 before starting this owner.
"""
import argparse
from dataclasses import asdict
import json
import os
from pathlib import Path
import signal
import sys
import time
import zlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE));sys.path.insert(0,str(HERE.parent/'uio1'))
from launch import Context,private_json
from observe import Observer,GuiWitness
from capture import selected_window
from x11 import X11,summaries
from xrecord import Recorder
from popup import Editor,WindowGraph,bind,SIZE,SurfaceObserver
from desktop import PopupX11,SurfaceGraphX11,GroupX11,click_pair
from transient import Transaction,input_receipt,stable
from executor import Surface

CLASS='41727475415649534B61743150726F63'
PROFILE='2fd865a9a91ae7fae7215a0f8e9c0d6fba717941be7ce6ed4a5a828f42ac5b66'

class Product:
    def __init__(self,c,out):
        self.c=c;self.out=out;self.stopped=None;self.helper=self.observer=self.graph=self.gui=self.x=self.record=self.popup_x=self.popup_record=None
        self.xgraph=None;self.transaction=None;self.group=None;self.action_receipts=[];self.xrecord_status=[]
        self.e=None;self.latest=None;self.previous=None;self.rows=[];self.gestures=[];self.inputs=[];self.clocks=[];self.frames=[];self.graphs=[];self.faults=[]
        self.began=time.monotonic();self.next_fault=0;self.next_clock=0;self.action=0
        if c.admission['profile_fingerprint']!=PROFILE:raise RuntimeError('UIO2 requires exact candidate 17')
        request=c.session.get('crash_capture') or {}
        if request.get('session')!=c.session['session'] or request.get('software')!=c.software:
            raise RuntimeError('CA1 not bound to this session/software')
        self.capture_status=Path(request['directory'])/'status.json'
        self.require_capture()
        process,windows,snapshot=c.census(out/'census.log');w,xid=selected_window(windows)
        for k in ('DISPLAY','XAUTHORITY'):
            if k in c.env:os.environ[k]=c.env[k]
        self.gui=GuiWitness(Path(c.session['directory'])/'ap11.ui',c.session['session'])
        self.gui.cursors=[max(0,self.gui.read(at)-512) for at in (64,80)]
        for r in self.gui.take():
            if r['lane']==1 and r['kind']==108:self.latest=r
        if not self.latest or self.latest['target']!=xid:raise RuntimeError('current native editor activation unavailable')
        self.e=Editor(c.session['session'],self.latest['native_view'],self.latest['activation'],snapshot['native']['generation'],
                      self.latest['editor_epoch'],process['pid'],process['start'],w['tid'],w['hwnd'],xid)
        self.e.validate();self.x=X11(xid);self.xgraph=SurfaceGraphX11(xid)
        unix_pid=self.x.property(xid,'_NET_WM_PID')
        if len(unix_pid)!=1:raise RuntimeError('XWayland Unix process identity unavailable')
        self.x.pid=unix_pid[0];self.x.check_identity()
        path=out/'observer.status'
        self.helper=c.helper('uio1-observer.exe',['observe-surfaces',self.e.hwnd,self.e.start,175,c.runtime.windows(path,c.prefix)],out/'observer.log',180)
        end=time.monotonic()+10
        while not Path(str(path)+'.windows').exists() or Path(str(path)+'.windows').stat().st_size!=SIZE:
            self.helper.poll()
            if time.monotonic()>end:raise RuntimeError('window observer startup bound')
            time.sleep(.01)
        self.observer=SurfaceObserver(path,self.e.pid,self.e.start,self.e.hwnd)
        while not self.observer.status()['ready']:
            self.helper.poll()
            if time.monotonic()>end:raise RuntimeError('observer UI handshake bound')
            time.sleep(.01)
        self.graph=WindowGraph(Path(str(path)+'.windows'),self.e);self.record=Recorder(self.x)
        self.current();self.x.activate();private_json(out/'editor-private.json',asdict(self.e))
        self.fresh()

    def require_capture(self):
        status=json.loads(self.capture_status.read_text())
        if status.get('state')!='collecting' or status.get('capture_enabled') is not True or status.get('error') is not None or status.get('session')!=self.c.session['session']:
            raise RuntimeError('CA1 capture not actively collecting exact session')

    def stop(self,reason):self.stopped=self.stopped or reason

    def terminal(self):
        s=self.c.fault.snapshot()
        return bool(s.get('terminal_instance') or s.get('result_status',{}).get('rejection') or s.get('editor',{}).get('failure') or s.get('editor',{}).get('closed'))

    def current(self):
        if self.stopped:raise RuntimeError('UIO2 stopped: '+self.stopped)
        if time.monotonic()-self.began>170:raise RuntimeError('UIO2 session time bound')
        self.tick()
        s=self.c.fault.snapshot();r=self.latest
        if (s.get('terminal_instance') or s.get('result_status',{}).get('rejection') or not s.get('editor',{}).get('open') or
            s.get('editor',{}).get('failure') or not r or r.get('result') or
            (r['native_view'],r['activation'],r['editor_epoch'],r['target']) !=
            (self.e.native_view,self.e.activation,self.e.epoch,self.e.xid) or
            s['native']['generation']!=self.e.generation):
            self.stop('terminal or editor identity changed');raise RuntimeError(self.stopped)
        return self.e

    def tick(self):
        if self.helper:self.helper.poll()
        if self.record:self.record.poll()
        if self.popup_record:self.popup_record.poll()
        if self.observer:
            self.rows.extend(self.observer.take());s=self.observer.status()
            if any(s[k] for k in ('dropped','heartbeat_errors','scope_errors','unhook_errors')):raise RuntimeError('observer evidence incomplete')
            if time.monotonic()>=self.next_clock:
                self.clocks.append(self.observer.clock());self.next_clock=time.monotonic()+2
        if self.gui:
            for r in self.gui.take():
                if r['lane']==1 and r['kind']==108:self.latest=r
                if len(self.gestures)<4096:self.gestures.append(dict(r,action=self.action))
                else:raise RuntimeError('gesture witness bound')
        if time.monotonic()>=self.next_fault:
            if len(self.faults)>=1800:raise RuntimeError('fault witness bound')
            self.faults.append(self.c.fault.snapshot());self.next_fault=time.monotonic()+.1

    def fresh(self):
        self.current();g=self.xgraph.enrich(self.graph.snapshot(),self.terminal());self.current()
        if len(self.graphs)>=128:raise RuntimeError('window graph retention bound')
        self.graphs.append(g)
        return g

    def target(self,g):
        # A same-process top-level window with no GW_OWNER is not an editor
        # popup authority. It must nevertheless prevent falling back to an
        # underlying editor drawable as if that were the visible menu. The
        # exact owner-chain binder below refuses it; no position/name inference.
        visible=[r for r in g['windows'] if r['hwnd']!=self.e.hwnd and r['root']==r['hwnd'] and r['visible']]
        if not visible:return next(r for r in g['windows'] if r['hwnd']==self.e.hwnd),self.x
        from popup import owned
        rows={r['hwnd']:r for r in g['windows']}
        canonical=[r for r in visible if owned(r,rows,self.e)]
        if canonical:
            row=bind(g,self.e,self.current());group=None
        else:
            if self.group is None:
                if self.transaction is None:raise RuntimeError('ownerless popup has no action transaction')
                self.group=self.transaction.bind()
            self.group.revalidate(g,self.current(),self.rows);group=self.group;row=group.target
        if self.popup_x is None or stable(self.popup_x.row)!=stable(row):
            if self.popup_record:
                self.popup_record.close();self.inputs.extend(self.popup_record.records)
                self.xrecord_status.append(dict(dropped=self.popup_record.dropped,unparsed=self.popup_record.unparsed));self.popup_record=None
            if self.popup_x:self.popup_x.close();self.popup_x=None
            self.popup_x=(GroupX11(group,self.e,self.fresh,self.current,self.terminal,lambda:self.rows,self.xgraph) if group else
                          PopupX11(row,self.e,self.fresh,self.current,self.terminal))
            self.popup_record=Recorder(self.popup_x)
        return row,self.popup_x

    def frame(self):
        self.current();g=self.fresh();row,x=self.target(g);meta,pixels=x.capture()
        if len(self.frames)>=24:raise RuntimeError('private frame capacity')
        facts,_=summaries(pixels,meta['width'],meta['height'],meta['stride'])
        n=len(self.frames)+1;self.frames.append(dict(meta=meta,facts=facts,window=row,action=self.action,group_identity=self.group.identity if self.group else '',group_authority=self.group.authority if self.group else None))
        folder=self.out/'frames';folder.mkdir(mode=0o700,exist_ok=True)
        private_json(folder/f'{n}.json',self.frames[-1])
        with os.fdopen(os.open(folder/f'{n}.bgra.z',os.O_CREAT|os.O_EXCL|os.O_WRONLY,0o600),'wb') as f:f.write(zlib.compress(pixels,3))
        return meta,pixels

    def act(self,capsule,number):
        self.require_capture();self.current();g=self.fresh();row,x=self.target(g)
        if capsule.group_identity != (self.group.identity if self.group else ''):raise RuntimeError('capsule group transaction changed')
        if stable(row)!=stable(capsule.target):raise RuntimeError('capsule target changed')
        if capsule.action=='open_menu' and row['hwnd']!=self.e.hwnd:raise RuntimeError('unexpected popup before menu action')
        if capsule.action in ('resize_window','resize_choice') and row['hwnd']==self.e.hwnd:raise RuntimeError('owned popup required')
        if capsule.action=='dismiss_transient_group':
            if self.group is None:raise RuntimeError('dismissal requires exact group authority')
            group=self.group;group.revalidate(g,self.current(),self.rows)
            self.action=number;self.observer.action(number)
            # Existing XTEST non-text key owner; only the exact active editor or
            # content window can receive this custodian-authorized Escape.
            active=self.x.property(self.x.root,'_NET_ACTIVE_WINDOW')
            keyowner=self.x if active==[self.e.xid] else x
            keyowner.key(True,'escape')
            try:self.tick()
            finally:keyowner.key(False,'escape')
            end=time.monotonic()+.5
            while time.monotonic()<end:self.current();time.sleep(.005)
            final=self.fresh();ids={r['hwnd'] for r in group.members}
            if ({r['hwnd'] for r in final['windows'] if r['hwnd']!=self.e.hwnd and r['root']==r['hwnd'] and r['visible']} != group.baseline_peers-ids):raise RuntimeError('group dismissal incomplete; operator handling required')
            self.group=None;self.transaction=None
            private_json(self.out/'actions'/f'{number}.json',dict(kind='dismiss_transient_group',group=group.authority,after=final))
            return dict(input_sent=True,all_members_hidden=True,technical_verdict='custodian_pending')
        rx,ry,w,h=x.geometry();px,py=capsule.point
        if not rx<=px<rx+w or not ry<=py<ry+h:raise RuntimeError('capsule point outside exact drawable')
        prior_group=self.group
        self.action=number;self.observer.action(number);self.record.action=number
        if self.popup_record:self.popup_record.action=number
        self.clocks.append(self.observer.clock());before=self.c.fault.snapshot()
        motion=x.move(((px-rx)/(w-1),(py-ry)/(h-1)));settled=x.settle_pointer()
        # Revalidate after compositor motion acknowledgement and before Down.
        self.current();nowrow,_=self.target(self.fresh())
        if stable(nowrow)!=stable(row):raise RuntimeError('target changed before Down')
        if isinstance(x,GroupX11):x.verify_point(capsule.point)
        def held():
            end=time.monotonic()+.06
            while time.monotonic()<end:self.tick();time.sleep(.005)
            if prior_group:prior_group.revalidate(self.fresh(),self.current(),self.rows)
        down,up=click_pair(x,held)
        end=time.monotonic()+.3
        while time.monotonic()<end:
            self.current()
            time.sleep(.005)
        after=self.c.fault.snapshot();after_graph=self.fresh();self.tick()
        all_x=self.inputs+(self.record.records if self.record else [])+(self.popup_record.records if self.popup_record else [])
        # Recorder instances cover different exact windows; deduplicate the
        # identical X-server observation if both connections saw the same event.
        unique={json.dumps(r,sort_keys=True):r for r in all_x};all_x=list(unique.values())
        receipt_evidence=input_receipt(number,all_x,self.rows,all(r.dropped==0 for r in (self.record,self.popup_record) if r))
        self.action_receipts.append(receipt_evidence)
        self.transaction=Transaction(self.e,number,g,receipt_evidence,after_graph,self.rows,prior_group)
        self.group=None
        # Binding is a custodian operation after this one input action. A new
        # nested group never gains an automatic follow-up capsule.

        receipt=dict(action=number,kind=capsule.action,motion=motion,settlement=settled,down=down,up=up,before=before,after=after,windows=after_graph)
        private_json(self.out/'actions'/f'{number}.json',receipt)
        return dict(input_sent=True,technical_verdict='custodian_pending')

    def close(self):
        errors=[]
        try:self.tick()
        except Exception as e:errors.append(type(e).__name__)
        final=self.c.fault.snapshot()
        cleanup=None;observer_status=None
        if self.observer:
            self.observer.stop();end=time.monotonic()+3
            while not self.observer.status()['closed'] and time.monotonic()<end:
                if self.helper:self.helper.poll()
                time.sleep(.01)
            self.rows.extend(self.observer.take());observer_status=self.observer.status()
        if self.helper:
            try:cleanup=self.helper.finish()
            except Exception as e:errors.append(type(e).__name__)
        for record in (self.popup_record,self.record):
            if record:
                record.poll();self.inputs.extend(record.records);self.xrecord_status.append(dict(dropped=record.dropped,unparsed=record.unparsed));record.close()
        for obj in (self.popup_x,self.xgraph,self.x,self.graph,self.observer,self.gui):
            if obj:obj.close()
        private_json(self.out/'result-private.json',dict(schema=1,editor=asdict(self.e) if self.e else None,stopped=self.stopped,
          win32=self.rows,xrecord=self.inputs,gestures=self.gestures,clocks=self.clocks,frames=self.frames,
          graphs=self.graphs,action_receipts=self.action_receipts,xrecord_status=self.xrecord_status,group_authority=self.group.authority if self.group else None,faults=self.faults,final=final,observer_status=observer_status,helper_cleanup=cleanup,finalization_errors=errors,
          sdk_resize_request_result='not exported by immutable candidate 16/17; Win32 sizing and existing view stages retained separately'))
        self.c.fault.close()

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('admission',type=Path);p.add_argument('package',type=Path);p.add_argument('out',type=Path);p.add_argument('--port',type=int,default=8372);a=p.parse_args()
    os.umask(0o077);a.out.mkdir(mode=0o700)
    for name in ('capsules','actions'): (a.out/name).mkdir(mode=0o700)
    c=Context(a.admission,CLASS,a.package,if1=True);owner=None;surface=None
    try:
        owner=Product.__new__(Product);owner.__init__(c,a.out);surface=Surface(owner,a.out,a.port)
        private_json(a.out/'surface-private.json',dict(url='http://'+surface.host+'/'+surface.token,editor=asdict(owner.e)))
        signal.signal(signal.SIGTERM,lambda *_:owner.stop('custodian cancellation'))
        while not owner.stopped:
            owner.current();surface.http.handle_request()
    except Exception as e:
        if owner:owner.stop(type(e).__name__)
        private_json(a.out/'stop-private.json',dict(type=type(e).__name__,detail=str(e)))
    finally:
        if surface:surface.close();private_json(a.out/'executor-receipts.json',surface.receipts)
        if owner:owner.close()
        else:c.fault.close()

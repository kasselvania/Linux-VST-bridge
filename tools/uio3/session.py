#!/usr/bin/env python3
"""Human-only two-action observer. No coordinates or input operations accepted."""
import argparse
import dataclasses
import json
import os
import pathlib
import signal
import sys
import time
HERE=pathlib.Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent/'uio1'))
sys.path.insert(0,str(HERE.parent/'uio2'))
from launch import Context,private_json
from observe import Observer,GuiWitness,SIZE
from capture import selected_window
from popup import Editor,WindowGraph
from x11 import X11,summaries
from xi2 import XI2,XIRecorder
from timeline import summarize,readable

CLASS='41727475415649534B61743150726F63'
PROFILE='56872b858daef293aa47c0fcbd7e99e6c586e1311366fedbab0bb410fb0f55cd'

class Labels:
    def __init__(self):self.value=0
    def accept(self,value):
        if type(value) is not int or value not in (0,1,2,3):raise ValueError('closed action label')
        if value==self.value:return False
        if value==3 or value==self.value+1:self.value=value;return True
        raise ValueError('no repeat, reverse or skipped action')

def finish(h):
    r=h.finish();c=r.get('cleanup',{})
    if r.get('exit')!=0 or r.get('overflow')!=0 or c.get('owned_descendants_zero') is not True or c.get('process_group_empty') is not True:
        raise RuntimeError('observer helper cleanup incomplete')
    return r

class Session:
    def __init__(self,c,out):
        self.c=c;self.out=out;self.handles=[];self.stop=False;self.labels=Labels();self.next_frame=0;self.next_clock=0;self.next_device=0;self.previous=None
        self.x=self.xi=self.rec=self.obs=self.gui=self.graph=self.helper=None
        self.raw=dict(schema=1,profile_fingerprint=c.admission['profile_fingerprint'],actions=[],raw=[],x11=[],win32=[],gui=[],frames=[],brackets=[],drops={},cleanup={},completed=False)
        self.last_gui_poll=time.monotonic_ns();self.start=time.monotonic();self.next_identity=0
    def open(self):
        c=self.c
        if c.admission['profile_fingerprint']!=PROFILE:raise RuntimeError('ordinary Pigments18 required')
        process,rows,snap=c.census(self.out/'census.log');w,xid=selected_window(rows)
        for k in ('DISPLAY','XAUTHORITY'):
            if k in c.env:os.environ[k]=c.env[k]
        self.gui=GuiWitness(pathlib.Path(c.session['directory'])/'ap11.ui',c.session['session'])
        self.gui.cursors=[max(0,self.gui.read(at)-512) for at in (64,80)]
        latest=[r for r in self.gui.take() if r['lane']==1 and r['kind']==108]
        if not latest or latest[-1]['target']!=xid:raise RuntimeError('native view identity unavailable')
        self.latest=latest[-1];r=self.latest
        self.e=Editor(c.session['session'],r['native_view'],r['activation'],snap['native']['generation'],r['editor_epoch'],process['pid'],process['start'],w['tid'],w['hwnd'],xid)
        self.e.validate();self.raw['editor']=dataclasses.asdict(self.e);self.raw['before']=snap
        self.x=X11(xid);pid=self.x.property(xid,'_NET_WM_PID')
        if len(pid)!=1:raise RuntimeError('Linux editor process identity unavailable')
        self.x.pid=pid[0];self.x.check_identity();self.raw['initial_geometry']=self.x.geometry()
        path=self.out/'observer.status'
        self.helper=c.helper('uio1-observer.exe',['observe-input',w['hwnd'],process['start'],175,c.runtime.windows(path,c.prefix)],self.out/'observer.log',185)
        end=time.monotonic()+10
        while not path.exists() or path.stat().st_size!=SIZE:
            self.helper.poll()
            if time.monotonic()>end:raise RuntimeError('observer startup expired')
            time.sleep(.02)
        self.obs=Observer(path,process['pid'],process['start'],w['hwnd'])
        while not self.obs.status()['ready']:
            self.helper.poll()
            if time.monotonic()>end:raise RuntimeError('observer handshake expired')
            time.sleep(.02)
        # Header extension is independent of audio/GUI protocol versions.
        if self.obs.read(232)!=1:raise RuntimeError('touch observer mode missing')
        self.graph=WindowGraph(pathlib.Path(str(path)+'.windows'),self.e)
        g=self.graph.snapshot();windows=g['windows'] if isinstance(g,dict) else g
        # Core/XI delivery is limited to proven XIDs on the exact window graph.
        targets={xid}
        for win in windows:
            if win.get('pid')==self.e.pid and win.get('tid')==self.e.tid and win.get('root')==self.e.hwnd and win.get('xid'):targets.add(win['xid'])
        self.xi=XI2(self.x,targets);self.rec=XIRecorder(self.x,self.xi.opcode,self.xi.devices,targets)
        self.raw['observer_package']=c.manifest
        self.raw['reporting_software']={k:v['sha256'] for k,v in c.software.items() if isinstance(v,dict) and 'sha256' in v}
        self.raw['devices']=self.xi.devices;self.raw['xi_version']=self.xi.version;self.raw['frequency']=self.obs.read(48)
        self.raw['capture_arm_at_launch']=bool(c.session.get('crash_capture'))
        private_json(self.out/'action.json',0)
        private_json(self.out/'ready.json',dict(schema=1,ready=True,labels={1:'without_held_note',2:'with_held_note',3:'stop'},maximum_seconds=160,input_injection=False))
    def collect(self):
        self.helper.poll();self.xi.poll();self.rec.devices=set(self.xi.devices);self.rec.poll()
        self.raw['win32'].extend(self.obs.take())
        before=self.last_gui_poll;now_ns=time.monotonic_ns()
        for r in self.gui.take():
            if r['lane']==1 and r['kind']==108:self.latest=r
            if len(self.raw['gui'])>=4096:self.raw['drops']['gui_capacity']=self.raw['drops'].get('gui_capacity',0)+1;continue
            self.raw['gui'].append(dict(r,action=self.labels.value,poll_before_ns=before))
        self.last_gui_poll=now_ns
        now=time.monotonic()
        if now>=self.next_clock:self.raw['brackets'].append(self.obs.clock());self.next_clock=now+2
        if now>=self.next_device:
            self.xi.refresh_devices()
            self.rec.devices=set(self.xi.devices)
            self.next_device=now+1
        s=self.c.fault.snapshot();r=self.latest
        if s.get('terminal_instance'):
            self.raw['terminal']=s['terminal_instance'];raise RuntimeError('terminal instance failure')
        if (s.get('editor',{}).get('failure') or not s.get('editor',{}).get('open') or
            (r['native_view'],r['activation'],r['editor_epoch'],r['target'])!=(self.e.native_view,self.e.activation,self.e.epoch,self.e.xid) or s['native']['generation']!=self.e.generation):raise RuntimeError('editor generation changed or closed')
        self.x.check_identity()
        if now>=self.next_frame:
            if self.x.geometry()!=tuple(self.raw['initial_geometry']):raise RuntimeError('editor geometry changed')
            meta,pixels=self.x.capture();facts,self.previous=summaries(pixels,meta['width'],meta['height'],meta['stride'],self.previous)
            if len(self.raw['frames'])<1200:self.raw['frames'].append(dict(meta,**facts,action=self.labels.value))
            else:self.raw['drops']['frames']=self.raw['drops'].get('frames',0)+1
            self.next_frame=now+.15
        status=self.obs.status()
        if any(status[k] for k in ('dropped','scope_errors','heartbeat_errors')):raise RuntimeError('Windows observer incomplete')
    def run(self):
        self.open()
        while not self.stop and time.monotonic()-self.start<160:
            # Mailbox is a private scalar label; never an input plan/coordinate.
            path=self.out/'action.json'
            if path.is_symlink() or path.stat().st_uid!=os.getuid() or path.stat().st_size>16:raise RuntimeError('action mailbox identity/size')
            value=json.loads(path.read_text())
            if value==3:
                self.collect() # Retain the previous action label through its final drain.
                self.labels.accept(value);self.raw['completed']=True;break
            if self.labels.accept(value):
                self.obs.action(value);self.rec.action=value;self.xi.action=value
                self.raw['actions'].append(dict(action=value,begin_ns=time.monotonic_ns()))
            self.collect()
            # Stop the first useful retained/missing release boundary after one
            # bounded three-second observation tail. This never issues input.
            releases=[r for r in self.xi.records if r['action']==self.labels.value and r['kind']=='raw_touch_end']+[r for r in self.rec.records if r['action']==self.labels.value and r['kind'] in ('xi_touch_end','core_up')]
            if releases and time.monotonic_ns()-max(r['observed_ns'] for r in releases)>3_000_000_000:
                provisional=dict(self.raw,raw=self.xi.records,x11=self.rec.records)
                analysis=summarize(provisional)['actions']
                if analysis and analysis[-1]['boundary'] in ('downstream_mouse_hook_nonzero','retrieved_release_rewritten_to_null','capture_clear_gesture_end_unobserved','wndproc_release_gesture_end_unobserved','core_release_without_win32_dispatch','xi_touch_end_without_core_or_win32_release'):
                    self.raw['stop']='first_useful_release_boundary';self.raw['completed']=True;break
            time.sleep(.005)
        if not self.raw['completed']:self.raw['stop']='cancellation_or_time_bound'
    def close(self):
        errors=[];self.raw['observation_end_ns']=time.monotonic_ns()
        # Stop read-only observation; never release human input or kill vendor.
        if self.rec:
            try:self.rec.close()
            except Exception as e:errors.append(type(e).__name__)
            self.raw['x11']=self.rec.records;self.raw['drops']['xrecord']=self.rec.dropped
        if self.xi:
            self.raw['device_history']=self.xi.device_history;self.raw['raw']=self.xi.records;self.raw['xi_counters']=self.xi.counters;self.raw['drops']['xi_raw']=self.xi.dropped
            try:self.xi.close()
            except Exception as e:errors.append(type(e).__name__)
        if self.obs:
            try:self.obs.action(0);self.obs.stop()
            except Exception as e:errors.append(type(e).__name__)
        if self.helper:
            try:self.raw['cleanup']['helper']=finish(self.helper)
            except Exception as e:errors.append(type(e).__name__)
        if self.obs:
            try:
                self.raw['win32'].extend(self.obs.take());s=self.obs.status();self.raw['observer_status']=s;self.raw['drops']['win32']=s['dropped']
                if s['closed']!=1 or s['detached']!=1 or s['unhook_errors']:errors.append('observer_detachment_unconfirmed')
            except Exception as e:errors.append(type(e).__name__)
        if self.gui:self.raw['drops']['gui_ring']=sum(self.gui.dropped)
        for obj in (self.graph,self.obs,self.gui,self.x):
            if obj:
                try:obj.close()
                except Exception as e:errors.append(type(e).__name__)
        try:self.raw['after']=self.c.fault.snapshot()
        except Exception as e:errors.append(type(e).__name__)
        finally:
            try:self.c.fault.close()
            except Exception as e:errors.append(type(e).__name__)
        self.raw['cleanup']['errors']=errors
        if errors:self.raw['completed']=False
        private_json(self.out/'timeline.json',self.raw)
        public=summarize(self.raw);private_json(self.out/'summary.json',public)
        fd=os.open(self.out/'summary.txt',os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
        with os.fdopen(fd,'w') as f:f.write(readable(public))

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('admission',type=pathlib.Path);p.add_argument('package',type=pathlib.Path);p.add_argument('out',type=pathlib.Path);a=p.parse_args()
    c=Context(a.admission,CLASS,a.package,manifest_path=HERE/'package.json')
    a.out.mkdir(mode=0o700,exist_ok=False);s=Session(c,a.out)
    signal.signal(signal.SIGTERM,lambda *_:setattr(s,'stop',True));signal.signal(signal.SIGINT,lambda *_:setattr(s,'stop',True))
    try:s.run()
    except Exception as e:s.raw['stop']=str(e);raise
    finally:s.close()

#!/usr/bin/env python3
"""Read-only exact managed-editor renderer observation.

The Rust command admits only the current retained ManagedExperimental
publication. This collector sends no input, activation, resize, SDK, or audio
request. HWNDs, process/session identities, and raw frames remain private.
"""
import argparse
from collections import Counter
import dataclasses
import json
import os
import pathlib
import signal
import sys
import time
import zlib

HERE=pathlib.Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
sys.path.insert(0,str(HERE.parent/'uio2'))
from launch import Context,private_json
from observe import Observer,GuiWitness,SIZE
from capture import selected_window,renderer
from popup import Editor,WindowGraph
from desktop import SurfaceGraphX11,unobscured
from x11 import X11,summaries

GRAPH_SIZE=4096+2*(32+128*192)
MAXIMUM_SECONDS=30

def frame_metrics(meta,pixels):
    facts,_=summaries(pixels,meta['width'],meta['height'],meta['stride'])
    samples=[]
    for y in range(0,meta['height'],8):
        row=y*meta['stride']
        for x in range(0,meta['width'],8):
            b,g,r=pixels[row+x*4:row+x*4+3];samples.append((r,g,b))
    if not samples:raise RuntimeError('empty drawable sample')
    n=len(samples);channels=list(zip(*samples))
    facts.update(sampled_pixels=n,rgb_min=[min(c) for c in channels],rgb_max=[max(c) for c in channels],
        rgb_mean=[sum(c)/n for c in channels],near_white_percent=100*sum(min(p)>=245 for p in samples)/n,
        near_black_percent=100*sum(max(p)<=10 for p in samples)/n,distinct_sampled_rgb=len(set(samples)))
    return facts

def record_metrics(rows,frequency):
    counts=Counter((r['source'],r['message']) for r in rows)
    heartbeats=[r for r in rows if r['source']==4]
    return dict(records=len(rows),wm_paint_records=sum(n for (source,message),n in counts.items() if message==15),
        wm_paint_by_source=[dict(source=s,count=n) for (s,m),n in sorted(counts.items()) if m==15],
        heartbeat_records=len(heartbeats),
        maximum_heartbeat_ms=max((r['result']*1000/frequency for r in heartbeats),default=None),
        interpretation='message counts are observations only; zero WM_PAINT can mean a retained surface or no invalid region, and heartbeat continuity does not establish WM_PAINT delivery')

def graph_metrics(graph,editor):
    root=next((r for r in graph['windows'] if r['hwnd']==editor.hwnd),None)
    if not root:raise RuntimeError('exact editor root absent from graph')
    children=[r for r in graph['windows'] if r['hwnd']!=editor.hwnd and r['root']==editor.hwnd]
    def extent(r):return [r['client'][2]-r['client'][0],r['client'][3]-r['client'][1]]
    return dict(root={k:root[k] for k in ('visible','enabled','minimized','dpi','style','exstyle','rect','client')},
        child_count=len(children),visible_child_count=sum(bool(r['visible']) for r in children),
        zero_client_children=sum(extent(r)[0]<=0 or extent(r)[1]<=0 for r in children),
        child_interpretation='count and geometry only; zero child HWNDs does not imply failed drawing into the supplied parent',
        children=[dict(visible=bool(r['visible']),enabled=bool(r['enabled']),minimized=bool(r['minimized']),
            dpi=r['dpi'],style=r['style'],exstyle=r['exstyle'],rect=r['rect'],client=r['client'],client_extent=extent(r)) for r in children])

class Session:
    def __init__(self,context,out,deadline):
        self.c=context;self.out=out;self.deadline=deadline;self.stop=False
        self.helper=self.obs=self.graph=self.gui=self.x=self.xgraph=None
        self.latest=None;self.editor=None;self.records=[];self.frames=[];self.graphs=[]
        self.frequency=0
        self.result=dict(schema=1,profile_fingerprint=context.admission['profile_fingerprint'],
            admission='current_retained_managed_experimental',maximum_seconds=MAXIMUM_SECONDS,
            input_injection=False,activation_request=False,resize_request=False,desktop_capture=False,
            capture_available=False,completed=False)

    def check_time(self):
        if self.stop:raise InterruptedError('observation cancelled')
        if time.monotonic()>=self.deadline:raise TimeoutError('renderer observation deadline')

    def poll_identity(self):
        self.check_time();self.helper.poll();self.records.extend(self.obs.take())
        status=self.obs.status()
        if any(status[k] for k in ('dropped','heartbeat_errors','scope_errors','unhook_errors')):
            raise RuntimeError('observer evidence incomplete')
        for row in self.gui.take():
            if row['lane']==1 and row['kind']==108:self.latest=row
        snap=self.c.fault.snapshot();r=self.latest;e=self.editor
        if (snap.get('terminal_instance') or snap.get('result_status',{}).get('rejection') or
            not snap.get('editor',{}).get('open') or snap.get('editor',{}).get('failure') or
            not r or r.get('result') or
            (r['native_view'],r['activation'],r['editor_epoch'],r['target'])!=(e.native_view,e.activation,e.epoch,e.xid) or
            snap.get('native',{}).get('generation')!=e.generation):
            raise RuntimeError('editor identity changed or failed')
        self.x.check_identity();return snap

    def open(self):
        process,windows,snap=self.c.census(self.out/'census.log');w,xid=selected_window(windows)
        for key in ('DISPLAY','XAUTHORITY'):
            if key in self.c.env:os.environ[key]=self.c.env[key]
        self.gui=GuiWitness(pathlib.Path(self.c.session['directory'])/'ap11.ui',self.c.session['session'])
        self.gui.cursors=[max(0,self.gui.read(at)-512) for at in (64,80)]
        rows=[r for r in self.gui.take() if r['lane']==1 and r['kind']==108]
        if not rows or rows[-1]['target']!=xid or rows[-1].get('result'):
            raise RuntimeError('current native editor identity unavailable')
        self.latest=rows[-1];r=self.latest
        self.editor=Editor(self.c.session['session'],r['native_view'],r['activation'],snap['native']['generation'],
            r['editor_epoch'],process['pid'],process['start'],w['tid'],w['hwnd'],xid)
        self.editor.validate();self.result['before']=snap;self.result['initial_census']=windows
        self.x=X11(xid);pids=self.x.property(xid,'_NET_WM_PID')
        if len(pids)!=1:raise RuntimeError('exact Linux editor process unavailable')
        self.x.pid=pids[0];self.x.check_identity();self.xgraph=SurfaceGraphX11(xid)
        self.result['renderer_modules']=dict(windows=[r['kind'] for r in windows if r['type']=='renderer_module'],linux=renderer(self.x.pid),
            windows_census_complete=not any(r['type']=='module_census_unavailable' for r in windows))
        path=self.out/'observer.status';remaining=max(1,min(25,int(self.deadline-time.monotonic())))
        self.helper=self.c.helper('uio1-observer.exe',['observe',w['hwnd'],process['start'],remaining,
            self.c.runtime.windows(path,self.c.prefix)],self.out/'observer.log',remaining)
        while (not path.exists() or path.stat().st_size!=SIZE or not pathlib.Path(str(path)+'.windows').exists()
               or pathlib.Path(str(path)+'.windows').stat().st_size!=GRAPH_SIZE):
            self.check_time();self.helper.poll();time.sleep(.01)
        self.obs=Observer(path,process['pid'],process['start'],w['hwnd'])
        while not self.obs.status()['ready']:
            self.check_time();self.helper.poll();time.sleep(.01)
        self.frequency=self.obs.read(48)
        if not self.frequency:raise RuntimeError('observer clock unavailable')
        self.graph=WindowGraph(pathlib.Path(str(path)+'.windows'),self.editor)
        self.poll_identity()

    def fresh_graph(self):
        graph=self.xgraph.snapshot(self.graph.snapshot,
            lambda:bool(self.c.fault.snapshot().get('terminal_instance')))
        root=next((r for r in graph['windows'] if r['hwnd']==self.editor.hwnd),None)
        xfacts=graph['x11'].get(str(self.editor.xid))
        if not root or not xfacts or not root['visible'] or root['minimized'] or not xfacts['viewable']:
            raise RuntimeError('exact editor drawable unavailable')
        frame,stack=self.xgraph.stack();unobscured(stack,frame,xfacts['rect'])
        if frame!=xfacts['frame']:raise RuntimeError('exact editor X11 frame changed')
        return graph,root,xfacts

    def sample(self,label):
        before=self.poll_identity();graph,root,xfacts=self.fresh_graph()
        meta,pixels=self.x.capture();after_graph,after_root,after_xfacts=self.fresh_graph();after=self.poll_identity()
        win_fields=('hwnd','pid','tid','root','root_owner','xid','visible','enabled','minimized','dpi',
            'style','exstyle','class_atom','class_hash','rect','client')
        x_fields=('frame','parent_chain','rect','viewable','override_redirect','transient_for','window_type',
            'wm_state','net_wm_pid','bounding_shape','input_shape')
        if (any(root[k]!=after_root[k] for k in win_fields) or
            any(xfacts[k]!=after_xfacts[k] for k in x_fields)):
            raise RuntimeError('exact editor drawable changed during capture')
        folder=self.out/'frames';folder.mkdir(mode=0o700,exist_ok=True)
        with os.fdopen(os.open(folder/(label+'.bgra.z'),os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600),'wb') as f:
            f.write(zlib.compress(pixels,3))
        frame_row=dict(label=label,meta=meta,metrics=frame_metrics(meta,pixels),
            win32=graph_metrics(graph,self.editor),x11_root=xfacts,before=before,after=after,
            post_capture_graph=after_graph,post_capture_win32=graph_metrics(after_graph,self.editor),
            post_capture_x11_root=after_xfacts)
        private_json(folder/(label+'.json'),frame_row);self.frames.append(frame_row);self.graphs.append(graph)
        self.result['capture_available']=True

    def run(self):
        self.open();self.sample('first')
        end=min(self.deadline,time.monotonic()+2)
        while time.monotonic()<end:self.poll_identity();time.sleep(.01)
        self.sample('second');self.result['completed']=True

    def close(self,failure=None):
        errors=[];cleanup=None;status=None
        if self.obs:
            try:self.obs.stop()
            except Exception as e:errors.append(type(e).__name__)
        if self.helper:
            try:cleanup=self.helper.finish()
            except Exception as e:errors.append(type(e).__name__)
        if self.obs:
            try:self.records.extend(self.obs.take());status=self.obs.status()
            except Exception as e:errors.append(type(e).__name__)
        if cleanup and (cleanup.get('exit')!=0 or cleanup.get('overflow') or
            cleanup.get('cleanup',{}).get('owned_descendants_zero') is not True or
            cleanup.get('cleanup',{}).get('process_group_empty') is not True):
            errors.append('helper_cleanup_incomplete')
        if status and (status.get('closed')!=1 or status.get('detached')!=1 or
            status.get('dropped') or status.get('heartbeat_errors') or status.get('scope_errors') or
            status.get('unhook_errors')):
            errors.append('observer_cleanup_incomplete')
        after=None
        try:after=self.c.fault.snapshot()
        except Exception as e:errors.append(type(e).__name__)
        for obj in (self.graph,self.obs,self.gui,self.xgraph,self.x):
            if obj:
                try:obj.close()
                except Exception as e:errors.append(type(e).__name__)
        try:self.c.fault.close()
        except Exception as e:errors.append(type(e).__name__)
        self.result.update(frames=self.frames,graphs=self.graphs,win32=self.records,observer_status=status,
            helper_cleanup=cleanup,after=after,cleanup_errors=errors,failure=failure,frequency=self.frequency)
        if status:
            self.result['record_metrics']=record_metrics(self.records,self.frequency)
        private_json(self.out/'result-private.json',self.result)

def main():
    p=argparse.ArgumentParser();p.add_argument('admission',type=pathlib.Path);p.add_argument('class_id')
    p.add_argument('package',type=pathlib.Path);p.add_argument('observation_id');a=p.parse_args()
    if len(a.observation_id)!=32 or any(c not in '0123456789abcdef' for c in a.observation_id):
        raise SystemExit('renderer observation refused')
    began=time.monotonic()
    try:c=Context(a.admission,a.class_id,a.package,managed_observation=True)
    except BaseException:
        print('Renderer observation refused.',file=sys.stderr);return 1
    if c.admission.get('maximum_seconds')!=MAXIMUM_SECONDS:
        c.fault.close();print('Renderer observation refused.',file=sys.stderr);return 1
    out=c.package/('renderer-observation-'+a.observation_id);out.mkdir(mode=0o700,exist_ok=False)
    s=Session(c,out,began+MAXIMUM_SECONDS);failure=None
    signal.signal(signal.SIGTERM,lambda *_:setattr(s,'stop',True));signal.signal(signal.SIGINT,lambda *_:setattr(s,'stop',True))
    try:
        s.run()
    except BaseException as e:
        failure=dict(type=type(e).__name__,detail=str(e));s.result['completed']=False
    finally:
        s.close(failure)
    if failure or s.result['cleanup_errors'] or not s.result['completed']:
        print('Renderer observation refused; inspect private result.',file=sys.stderr);return 1
    print(json.dumps(dict(completed=True,frames=len(s.frames),graphs=len(s.graphs),input_injection=False)))
    return 0

if __name__=='__main__':raise SystemExit(main())

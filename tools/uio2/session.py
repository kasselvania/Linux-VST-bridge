"""One generated XWayland action through the production VendorView resize path.
No operator display, authorized prefix, vendor code or arbitrary input.
"""
import importlib.util
import json
import os
from pathlib import Path
import sys
import time
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
sys.path.insert(0,str(HERE.parent/'uio1'))
sys.path.insert(0,str(HERE.parent/'uir1'))
sys.path.insert(0,str(HERE.parents[1]/'bridge-manager/runtime'))
from isolation import runner_environment
import ownership
from launch import Helper,private_json,sealed_bytes
from observe import Mapping,Observer
from x11 import X11,summaries
from xrecord import Recorder
from popup import Editor,WindowGraph,bind,Capsule,Permit,SIZE
from desktop import PopupX11
spec=importlib.util.spec_from_file_location('uir1_session',HERE.parent/'uir1/session.py')
uir1=importlib.util.module_from_spec(spec);spec.loader.exec_module(uir1)
finish=uir1.finish_helper

FIELDS=dict(pid=16,start=24,tid=32,root=40,child=48,popup=56,lookalike=64,ready=72,
 request=88,result=96,on_size=104,paints=112,down=120,up=128,requested_width=136,
 requested_height=144,parent_width=152,parent_height=160,child_width=168,child_height=176,
 closed=184,error=192,view_stage=200)

def run(root,package):
    os.umask(0o077);env=runner_environment(root,os.environ)
    runner=json.loads((root/'runner.json').read_text());files=json.loads((root/'package.json').read_text())
    target=root/'compatdata/pfx/drive_c/uio2';target.mkdir(parents=True,mode=0o700)
    for name,sha in files.items():
        with (target/name).open('xb') as f:f.write(sealed_bytes(package/name,sha))
        (target/name).chmod(0o500)
    base=[runner['entry_point'],'--verb=run','--',runner['proton']]
    report=dict(schema=1,completed=False,actions=[],win32=[],xrecord=[],clocks=[],frames=[])
    handles=[];status=observer=graph=x=record=popup=None
    def helper(name,args,seconds,label):
        h=Helper(ownership,[*base,'runinprefix','C:\\uio2\\'+name,*map(str,args)],env,target,root/(label+'.log'),seconds)
        handles.append(h);return h
    def tick():
        for h in handles:
            if not h.closed:h.poll()
        if record:record.poll()
        if observer:report['win32'].extend(observer.take())
    def wait(test,seconds):
        end=time.monotonic()+seconds
        while not test():
            tick()
            if time.monotonic()>=end:raise RuntimeError('bounded generated transition expired')
            time.sleep(.005)
        tick()
    def span(seconds):
        end=time.monotonic()+seconds
        while time.monotonic()<end:tick();time.sleep(.005)
    def snap():return{k:status.read(v) for k,v in FIELDS.items()}
    def click(target,point,action):
        observer.action(action);record.action=action
        report['clocks'].append(observer.clock())
        report['actions'].append(dict(action=action,motion=target.move(point),settlement=target.settle_pointer()))
        report['actions'].append(dict(action=action,down=target.button(True)));span(.06)
        report['actions'].append(dict(action=action,up=target.button(False)));span(.1)
    try:
        init=Helper(ownership,[*base,'getcompatpath',str(target)],env,target,root/'initialize.log',50);handles.append(init)
        finish(report,'initialization',init)
        fixture=helper('uio2-popup-fixture.exe',['C:\\uio2\\fixture.status'],125,'fixture')
        wait(lambda:(target/'fixture.status').exists() and (target/'fixture.status').stat().st_size==4096,50)
        status=Mapping(target/'fixture.status',4096);wait(lambda:status.read(72)==1,5)
        if status.read(0)!=0x324f4955 or status.read(8)!=1:raise RuntimeError('generated schema')
        initial=snap();report['before']=initial
        census=helper('uio1-observer.exe',['census',initial['pid']],10,'census');finish(report,'census_cleanup',census)
        rows=[json.loads(s) for s in (root/'census.log').read_text().splitlines() if s.startswith('{')]
        bindings=[r for r in rows if r.get('type')=='x11_binding' and r['hwnd']==initial['root']]
        if len(bindings)!=1 or not bindings[0]['xid']:raise RuntimeError('generated exact XWayland root missing')
        e=Editor('1'*32,1,1,1,1,initial['pid'],initial['start'],initial['tid'],initial['root'],bindings[0]['xid'])
        obs=helper('uio1-observer.exe',['observe',e.hwnd,e.start,90,'C:\\uio2\\observe.status'],100,'observer')
        wait(lambda:(target/'observe.status.windows').exists() and (target/'observe.status.windows').stat().st_size==SIZE,10)
        observer=Observer(target/'observe.status',e.pid,e.start,e.hwnd);wait(lambda:observer.status()['ready']==1,5)
        graph=WindowGraph(target/'observe.status.windows',e)
        x=X11(e.xid);x.activate();record=Recorder(x)
        # Main client location is derived from the exact generated child row.
        g=graph.snapshot();child=next(r for r in g['windows'] if r['hwnd']==initial['child'])
        rx,ry,rw,rh=x.geometry();cx,cy=child['client'][:2]
        click(x,((cx+40-rx)/(rw-1),(cy+25-ry)/(rh-1)),1)
        wait(lambda:status.read(56)!=0,3)
        first=graph.snapshot();row=bind(first,e,e);report['popup']=row
        if row['hwnd']!=status.read(56) or row['hwnd']==initial['lookalike']:raise RuntimeError('wrong generated popup')
        popup=PopupX11(row,e,graph.snapshot,lambda:e,lambda:bool(status.read(192)))
        m,pixels=popup.capture();facts,_=summaries(pixels,m['width'],m['height'],m['stride']);report['frames'].append(dict(meta=m,facts=facts))
        # Recorder follows the popup exact client; no global input adoption.
        record.close();report['xrecord'].extend(record.records);record=Recorder(popup)
        now=time.monotonic_ns();c=Capsule('generated-popup',e,row,'resize_window',(row['client'][0]+100,row['client'][1]+40),now,now+5_000_000_000)
        Permit(c).consume({'action':'resize_window'},e,time.monotonic_ns());report['capsule']=__import__('dataclasses').asdict(c)
        px,py,pw,ph=popup.geometry();click(popup,((c.point[0]-px)/(pw-1),(c.point[1]-py)/(ph-1)),2)
        wait(lambda:status.read(88)==1,3);span(.3)
        after=snap();report['after']=after
        if any(after[k]!=v for k,v in dict(request=1,result=0,on_size=1,requested_width=800,requested_height=500,parent_width=800,parent_height=500,child_width=800,child_height=500,view_stage=16).items()):
            raise RuntimeError('production resize result/geometry failed')
        if after['paints']<=initial['paints']:raise RuntimeError('no generated repaint')
        # Selection fails after the popup is removed, even with retained identity.
        try:bind(graph.snapshot(),e,e,row)
        except RuntimeError:report['closed_popup_refused']=True
        else:raise RuntimeError('closed popup still admitted')
        report['final_graph']=graph.snapshot();report['clocks'].append(observer.clock())
        # Actual server + Win32 receipts, not just an action log.
        report['xrecord'].extend(record.records);report['xrecord_status']=dict(dropped=record.dropped,unparsed=record.unparsed)
        for kind in (4,5):
            if not any(r['kind']==kind and r['action']==2 for r in report['xrecord']):raise RuntimeError('popup X RECORD receipt missing')
        for message in (0x201,0x202):
            if not any(r['message']==message and r['source']==5 and r['hwnd']==row['hwnd'] and r['action']==2 for r in report['win32']):raise RuntimeError('popup Win32 receipt missing')
        observer.stop();wait(lambda:observer.status()['closed']!=0,3);report['observer_status']=observer.status()
        if any(report['observer_status'][k] for k in ('dropped','heartbeat_errors','scope_errors','unhook_errors')):raise RuntimeError('observer incomplete')
        finish(report,'observer_cleanup',obs);status.write(80,9);wait(lambda:status.read(184)==1,3);finish(report,'fixture_cleanup',fixture)
        report['completed']=True
    except BaseException as exc:report['error']=str(exc)
    finally:
        if observer:observer.stop()
        if status:status.write(80,9)
        for i,h in enumerate(handles):
            if not h.closed:
                try:finish(report,'cleanup_'+str(i),h)
                except BaseException as exc:report['completed']=False;report['cleanup_error']=str(exc)
        if record:record.close()
        for obj in (popup,x,graph,observer,status):
            if obj:obj.close()
        private_json(root/'result-private.json',report)
    return 0 if report['completed'] else 1

if __name__=='__main__':sys.exit(run(Path(sys.argv[1]),Path(sys.argv[2])))

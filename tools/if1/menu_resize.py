#!/usr/bin/env python3
"""One private IF1 menu/resize observation, using exact UIO1 admission.

The driver submits at most two bounded normalized actions after inspecting the
single opened menu. Commands and snapshots occupy separate private directories.
No loop retries a click; failure ends this session and retains terminal custody.
"""
import argparse
import json
import pathlib
import signal
import sys
import time

sys.path.insert(0,str(pathlib.Path(__file__).resolve().parents[1]/'uio1'))
from launch import Context,private_json
from capture import Capture,Observer,Recorder,SIZE

CLASS='41727475415649534B61743150726F63'


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--admission-binary',type=pathlib.Path,required=True)
    p.add_argument('--package',type=pathlib.Path,required=True)
    p.add_argument('--output',type=pathlib.Path,required=True)
    a=p.parse_args();out=a.output;out.mkdir(mode=0o700)
    commands=out/'commands';commands.mkdir(mode=0o700)
    c=Context(a.admission_binary,CLASS,a.package,if1=True)
    process,rows,snapshot=c.census(out/'census.log');cap=Capture(c,out,process,rows,snapshot)
    signal.signal(signal.SIGTERM,lambda *_:setattr(cap,'stop_requested',True))
    signal.signal(signal.SIGINT,lambda *_:setattr(cap,'stop_requested',True))
    try:
        path=out/'ui.bin'
        cap.helper=c.helper('uio1-observer.exe',['observe',cap.w['hwnd'],process['start'],180,c.runtime.windows(path,c.prefix)],out/'observer.log',185)
        deadline=time.monotonic()+5
        while not path.exists() or path.stat().st_size!=SIZE:
            cap.helper.poll()
            if time.monotonic()>deadline:raise RuntimeError('observer mapping absent')
            time.sleep(.02)
        cap.observer=Observer(path,process['pid'],process['start'],cap.w['hwnd'])
        while not cap.observer.read(88):
            if time.monotonic()>deadline:raise RuntimeError('observer handshake absent')
            cap.helper.poll();time.sleep(.02)
        cap.record=Recorder(cap.x);cap.x.activate();cap.pump(.5,frames=False);cap.snapshot('before')
        cap.mark(1);cap.add(cap.x.move((25/1280,22/724)));cap.x.settle_pointer()
        cap.add(cap.x.button(True));cap.pump(.08,frames=False);cap.add(cap.x.button(False));cap.pump(.8,frames=False)
        cap.snapshot('menu');private_json(out/'menu-ready.json',{'ready':True})
        for n in (2,3):
            command=commands/('action-'+str(n)+'.json');deadline=time.monotonic()+60
            while not command.exists():
                cap.pump(.1,frames=False)
                if time.monotonic()>deadline:raise RuntimeError('bounded automatic plan wait')
            action=json.loads(command.read_text())
            if set(action)!={'kind','point'} or action['kind'] not in ('move','click'):raise ValueError('bounded action contract')
            cap.mark(n);cap.add(cap.x.move(action['point']));cap.x.settle_pointer()
            if action['kind']=='click':
                cap.add(cap.x.button(True));cap.pump(.08,frames=False);cap.add(cap.x.button(False))
            # Resizing invalidates the old spatial regions. Keep snapshots and
            # message/heartbeat facts; do not compare unlike pixel geometries.
            cap.pump(1,frames=False);cap.snapshot('action-'+str(n))
            private_json(out/('ready-'+str(n)+'.json'),{'ready':True})
        cap.pump(1,frames=False);cap.result['actions_completed']=True
    except BaseException as e:
        cap.result['failure']=str(e);cap.result['failure_class']=type(e).__name__
    finally:
        private_json(out/'terminal-before-detach.json',c.fault.snapshot())
        try:cap.close()
        except BaseException as e:
            private_json(out/'capture-finalization-error.json',{'class':type(e).__name__,'detail':str(e)})
            if not (out/'capture.json').exists():private_json(out/'capture.json',cap.result)
    print(json.dumps({'actions_completed':cap.result.get('actions_completed',False),'failure':cap.result.get('failure'),'output':str(out)}))


if __name__=='__main__':main()

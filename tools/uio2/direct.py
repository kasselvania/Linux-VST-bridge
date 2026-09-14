"""Observe one custodian capsule at a time using operator/Moonlight input.

This adapter never injects input. Unlike XTEST, it cannot interlock the external
executor's Down; stale/foreign input invalidates the observation and stops it.
A rolling bounded graph keeps the action transaction local to actual input,
not to the earlier time at which a model received its capsule.
"""
from collections import deque
from dataclasses import asdict
import argparse
import json
from pathlib import Path
import signal
import time
from product import Product,Context,CLASS,private_json
from executor import read_capsule
from transient import Transaction,input_receipt,stable,inside,MAX_ACTION_NS,validate_graph


def receipt_graph(history,receipt,editor,point,target):
    if [r['kind'] for r in receipt['x11']]!=[4,5] or [r['kind'] for r in receipt['win32']]!=[0x201,0x202]:
        raise RuntimeError('one external Down/Up pair required')
    xs=[r['observation'] for r in receipt['x11']]
    ws=[r['observation'] for r in receipt['win32']]
    if any(r['button']!=1 or r['state']&0x1fff != (0 if n==0 else 256) or max(abs(r['screen'][i]-point[i]) for i in (0,1))>2 for n,r in enumerate(xs)):
        raise RuntimeError('external point/button/modifier differs from capsule')
    if target['hwnd']!=editor.hwnd and any(r['hwnd']!=target['hwnd'] for r in receipt['win32']):
        raise RuntimeError('external input reached another popup')
    down=ws[0]['qpc'];xdown=xs[0]['observed_ns']
    eligible=[g for g in history if g['qpc']<down and g['interval_ns'][1]<xdown and
              any(stable(r)==stable(target) for r in g['windows'])]
    if not eligible:raise RuntimeError('no complete pre-input graph')
    return eligible[-1]


def post_action(editor,number,action,pre,receipt,post,rows,prior):
    a=validate_graph(pre,editor);b=validate_graph(post,editor)
    if stable(a)==stable(b):
        return Transaction(editor,number,pre,receipt,post,rows,prior)
    # A resize may change rectangles, never the editor identity. This record
    # is a geometry observation, not authority for a newly appearing popup.
    sa=stable(a);sb=stable(b)
    for key in ('rect','client'):sa.pop(key);sb.pop(key)
    if (action not in ('resize_window','resize_choice') or sa!=sb or
        not pre['commit']<post['commit'] or
        not 0<post['interval_ns'][1]-pre['interval_ns'][0]<=MAX_ACTION_NS or
        receipt.get('complete') is not True or receipt.get('action')!=number):
        raise RuntimeError('unexpected editor change after external action')
    return None


def run(owner,out):
    receipts=[];number=1;pending=None;history=deque(maxlen=16);first_down=None
    owner.frame()
    while not owner.stopped:
        owner.require_capture();owner.current()
        path=out/'capsules'/f'{number}.json'
        if pending is None and path.exists():
            if number>6:raise RuntimeError('direct capsule capacity')
            pending=read_capsule(path);pending.validate(owner.current(),time.monotonic_ns())
            graph=owner.fresh();target,x=owner.target(graph)
            if stable(target)!=stable(pending.target) or pending.group_identity!=(owner.group.identity if owner.group else ''):
                raise RuntimeError('direct capsule target changed')
            if pending.action not in ('open_menu','resize_window','resize_choice','known_control'):
                raise RuntimeError('direct capsule action unsupported')
            if pending.action=='open_menu' and target['hwnd']!=owner.e.hwnd:raise RuntimeError('unexpected popup before menu')
            if pending.action in ('resize_window','resize_choice') and target['hwnd']==owner.e.hwnd:raise RuntimeError('exact popup required')
            pointer=x.pointer()
            if pointer['mask']&0x1fff or not x.active_for_input(pointer):raise RuntimeError('held input/application focus')
            owner.action=number;owner.observer.action(number);owner.record.action=number
            if owner.popup_record:owner.popup_record.action=number
            history.clear();history.append(graph);first_down=None
            private_json(out/'ready'/f'{number}.json',dict(capsule=asdict(pending),adapter='Moonlight native input; observation only'))
        if pending:
            pending.validate(owner.current(),time.monotonic_ns())
            # Sample without retaining idle duplicates in the public report.
            graph=owner.xgraph.enrich(owner.graph.snapshot(),owner.terminal());owner.current()
            history.append(graph)
            xs=owner.inputs+owner.record.records+(owner.popup_record.records if owner.popup_record else [])
            xs=list({json.dumps(r,sort_keys=True):r for r in xs}.values())
            receipt=input_receipt(number,xs,owner.rows,all(r.dropped==0 for r in (owner.record,owner.popup_record) if r))
            if any(r['kind']==4 for r in receipt['x11']):
                if first_down is None:first_down=time.monotonic_ns()
                if time.monotonic_ns()-first_down>MAX_ACTION_NS:raise RuntimeError('external action receipt deadline')
            if len(receipt['x11'])>2 or len(receipt['win32'])>2:raise RuntimeError('extra external button action')
            if len(receipt['x11'])==len(receipt['win32'])==2:
                pre=receipt_graph(history,receipt,owner.e,pending.point,pending.target)
                # Retain a short bounded lifecycle interval after Up.
                end=time.monotonic()+.15
                while time.monotonic()<end:owner.current();time.sleep(.005)
                post=owner.fresh();owner.tick()
                prior=owner.group
                transaction=post_action(owner.e,number,pending.action,pre,receipt,post,owner.rows,prior)
                owner.transaction=transaction;owner.group=None;owner.action_receipts.append(receipt)
                row=dict(action=number,kind=pending.action,adapter='Moonlight native input',before=pre,windows=post,receipt=receipt,
                         external_down_interlock=False,technical_verdict='custodian_pending')
                private_json(out/'actions'/f'{number}.json',row);receipts.append(row)
                number+=1;pending=None;history.clear()
                owner.frame() # binds a resulting popup; never authorizes a next action
        time.sleep(.05)
    return receipts

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('admission',type=Path);p.add_argument('package',type=Path);p.add_argument('out',type=Path);a=p.parse_args()
    import os
    os.umask(0o077)
    a.out.mkdir(mode=0o700)
    for name in ('capsules','actions','ready'):(a.out/name).mkdir(mode=0o700)
    owner=None
    try:
        c=Context(a.admission,CLASS,a.package,if1=True)
        owner=Product.__new__(Product);owner.__init__(c,a.out)
        signal.signal(signal.SIGTERM,lambda *_:owner.stop('custodian cancellation'))
        run(owner,a.out)
    except Exception as e:
        if owner:owner.stop(type(e).__name__+': '+str(e))
        private_json(a.out/'stop-private.json',dict(type=type(e).__name__,detail=str(e)))
    finally:
        if owner:owner.close()

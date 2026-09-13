"""TSG1: action-bound, temporary authority for ownerless XWayland surfaces.

No product/name/count/size heuristic. The bounded gap is two desktop pixels,
and only exact consecutive X root stacking intervals qualify. A missing X Shape
input region is unavailable evidence, never an implicit full-window region.
"""
from dataclasses import asdict
import hashlib
import json
from popup import Editor, owned, inside

MAX_GAP = 2
MAX_ACTION_NS = 2_000_000_000

def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(',', ':')).encode()).hexdigest()

def stable(row):
    # Focus, cursor and z-order are sampled separately. Geometry and identities
    # cannot change while a capsule is in force.
    return {k:row[k] for k in ('hwnd','pid','tid','parent','owner','root','root_owner','xid','visible','enabled','minimized','dpi','style','exstyle','class_atom','class_hash','rect','client')}

def connected(a,b):
    return (max(a[0],b[0]) <= min(a[2],b[2])+MAX_GAP and
            max(a[1],b[1]) <= min(a[3],b[3])+MAX_GAP)

def components(rows):
    pending=list(rows);groups=[]
    while pending:
        group=[pending.pop()];changed=True
        while changed:
            changed=False
            for r in pending[:]:
                if any(connected(r['rect'],m['rect']) for m in group):
                    group.append(r);pending.remove(r);changed=True
        groups.append(group)
    return groups

def visible_top(r,e):
    return r['hwnd'] != e.hwnd and r['root']==r['hwnd'] and r['visible'] and not r['minimized']

def xfact(graph,row):
    x=graph['x11'].get(str(row['xid']))
    if (not x or x['xid']!=row['xid'] or x['viewable'] is not True or
        x['rect'] not in (row['rect'],row['client']) or x['input_shape'] is None or
        x['bounding_shape'] is None or not x['parent_chain'] or x['stack_index']<0):
        raise RuntimeError('surface XWayland identity/shape unavailable')
    return x

def validate_graph(g,e):
    e.validate()
    if g['editor']!=asdict(e) or g.get('terminal') is not False or g.get('complete') is not True:
        raise RuntimeError('incomplete/stale/terminal action graph')
    rows=g['windows']
    if len(rows)>128 or len({r['hwnd'] for r in rows})!=len(rows):raise RuntimeError('window graph extent/identity')
    root=next((r for r in rows if r['hwnd']==e.hwnd),None)
    if not root or root['pid']!=e.pid or root['tid']!=e.tid or root['xid']!=e.xid or not root['visible'] or not root['enabled']:
        raise RuntimeError('editor missing')
    return root

class Transaction:
    def __init__(self,editor,ordinal,before,receipt,after,lifecycle,prior_group=None):
        self.editor=editor;self.ordinal=ordinal;self.before=before;self.after=after
        if type(ordinal) is not int or not 1<=ordinal<=16:raise RuntimeError('action ordinal')
        a=validate_graph(before,editor);b=validate_graph(after,editor)
        if stable(a)!=stable(b):raise RuntimeError('editor changed during action')
        if (not before['commit']<after['commit'] or
            not 0 < after['interval_ns'][1]-before['interval_ns'][0] <= MAX_ACTION_NS):
            raise RuntimeError('action interval/commit bound')
        if receipt.get('action')!=ordinal or receipt.get('complete') is not True:
            raise RuntimeError('action input receipt incomplete')
        # Exact two-ended Down/Up, not the executor narrative. The receipt is
        # assembled by the custodian from the existing observer/X RECORD rows.
        for key,kinds in (('x11',(4,5)),('win32',(0x201,0x202))):
            rows=receipt[key]
            if [r['kind'] for r in rows]!=list(kinds) or any(r['action']!=ordinal for r in rows):
                raise RuntimeError('action Down/Up receipt/order')
        allowed={r['hwnd'] for r in before['windows'] if r['pid']==editor.pid and r['tid']==editor.tid and
                 (r['hwnd']==editor.hwnd or r['root']==editor.hwnd or r['root_owner']==editor.hwnd)}
        # A nested transaction may originate in an already admitted prior group.
        if prior_group is not None:
            prior_group.revalidate(before,editor,lifecycle)
            allowed.update(r['hwnd'] for r in prior_group.members)
        if any(r['hwnd'] not in allowed for r in receipt['win32']):raise RuntimeError('foreign action input')
        self.receipt=receipt
        self.lifecycle=[r for r in lifecycle if r['action']==ordinal and before['qpc']<=r['qpc']<=after['qpc']]
        if len(lifecycle)>16384:raise RuntimeError('lifecycle extent')

    def bind(self):
        e=self.editor;post=self.after;rows={r['hwnd']:r for r in post['windows']}
        canonical=[r for r in rows.values() if visible_top(r,e) and owned(r,rows,e)]
        if canonical:raise RuntimeError('canonical owner-chain route required')
        before={r['hwnd']:r for r in self.before['windows']};new=[];births={}
        for r in rows.values():
            if not visible_top(r,e):continue
            old=before.get(r['hwnd'])
            if old and old['visible']:
                # Existing peers cannot become newly authorized through movement.
                if stable(old)!=stable(r):raise RuntimeError('pre-existing peer changed during transaction')
                continue
            if r['pid']!=e.pid or r['tid']!=e.tid:raise RuntimeError('foreign new surface')
            if r['owner'] or r['root_owner']!=r['hwnd'] or not r['enabled'] or not r['xid']:
                raise RuntimeError('incomplete ownerless surface')
            events=[x for x in self.lifecycle if x['hwnd']==r['hwnd']]
            creates=[x for x in events if x['source']==9 and x['message']==3]
            shows=[x for x in events if (x['source']==10 and x['message']==0x8002) or
                   (x['source']==2 and x['message']==0x18 and x['buttons']==1)]
            dead=[x for x in events if (x['source']==9 and x['message']==4) or
                  (x['source']==10 and x['message'] in (0x8001,0x8003))]
            if dead or len(creates)>1 or not shows or (not old and len(creates)!=1):
                raise RuntimeError('surface create/show lifecycle missing or recycled')
            if old and (old['xid']!=r['xid'] or old['class_atom']!=r['class_atom'] or creates):
                raise RuntimeError('stale HWND/XID reuse')
            xfact(post,r);new.append(r);births[r['hwnd']]=[x['commit'] for x in events if x['source'] in (9,10)]
        plausible=[]
        for group in components(new):
            xs=[xfact(post,r) for r in group]
            indices=sorted(x['stack_index'] for x in xs)
            if len(set(indices))!=len(indices) or indices!=list(range(indices[0],indices[-1]+1)):
                raise RuntimeError('surface group stacking interval not unique/contiguous')
            bearing=[r for r,x in zip(group,xs) if x['input_shape']]
            if not bearing:continue # renderer-only group, not actionable
            if len(bearing)!=1:raise RuntimeError('surface group input member ambiguous')
            plausible.append((group,bearing[0]))
        if len(plausible)!=1:raise RuntimeError('transient group absent or ambiguous')
        members,target=plausible[0];members=sorted(members,key=lambda r:(r['hwnd'],r['xid']))
        authority=dict(editor=asdict(e),action=self.ordinal,pre_commit=self.before['commit'],
                       pre_hash=digest(self.before),input_hash=digest(self.receipt),post_interval=post['interval_ns'],
                       members=[stable(r) for r in members],lifetimes=births)
        return Group(e,authority,members,target,post)

class Group:
    def __init__(self,editor,authority,members,target,graph):
        self.editor=editor;self.authority=authority;self.identity=digest(authority)
        self.members=members;self.target=target;self.graph=graph
        self.baseline_peers={r['hwnd'] for r in graph['windows'] if visible_top(r,editor)}

    def revalidate(self,graph,current,lifecycle=()):
        if current!=self.editor:raise RuntimeError('stale group editor epoch')
        validate_graph(graph,self.editor)
        rows={r['hwnd']:r for r in graph['windows']}
        if {r['hwnd'] for r in rows.values() if visible_top(r,self.editor)}!=self.baseline_peers:
            raise RuntimeError('group membership changed; new transaction required')
        ids={r['hwnd'] for r in self.members}
        for ev in lifecycle:
            if ev['qpc']>self.graph['qpc'] and ev['hwnd'] in ids and ((ev['source']==9 and ev['message'] in (3,4)) or
                (ev['source']==10 and ev['message'] in (0x8000,0x8001,0x8003))):
                raise RuntimeError('group lifetime ended or HWND recycled')
        for old in self.members:
            new=rows.get(old['hwnd'])
            if not new or stable(new)!=stable(old):raise RuntimeError('group member changed')
            a=xfact(self.graph,old);b=xfact(graph,new)
            for key in ('xid','frame','parent_chain','rect','input_shape','bounding_shape','stack_index'):
                if a[key]!=b[key]:raise RuntimeError('group X identity/geometry/stack changed')
            if new['focus'] and new['focus'] not in ids and new['focus'] not in {r['hwnd'] for r in rows.values() if r['root']==self.editor.hwnd}:
                raise RuntimeError('group focus left application')
            if new['capture'] and new['capture'] not in ids and new['capture']!=self.editor.hwnd:
                raise RuntimeError('group capture left application')
        return self.target

    def target_at(self,point,first,second,pointer_xid,win_hwnd):
        self.revalidate(first,self.editor);self.revalidate(second,self.editor)
        matches=[]
        for r in self.members:
            x=xfact(second,r);origin=x['rect'][:2]
            if any(inside([a+origin[0],b+origin[1],c+origin[0],d+origin[1]],point) for a,b,c,d in x['input_shape']):matches.append(r)
        if len(matches)!=1 or matches[0]['hwnd']!=self.target['hwnd'] or pointer_xid!=self.target['xid'] or win_hwnd!=self.target['hwnd']:
            raise RuntimeError('unique input-bearing target not proved')
        return self.target


def input_receipt(action,xrecords,win32,complete=True):
    xs=[dict(kind=r['kind'],action=r['action'],observation=r) for r in xrecords if r['action']==action and r['kind'] in (4,5)]
    ws=[dict(kind=r['message'],action=r['action'],hwnd=r['hwnd'],observation=r) for r in win32
        if r['action']==action and r['source']==5 and r['message'] in (0x201,0x202)]
    return dict(action=action,complete=complete,x11=xs,win32=ws)

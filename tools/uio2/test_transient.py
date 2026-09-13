import copy
from dataclasses import asdict,replace
from pathlib import Path
import sys
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parent))
from transient import Transaction,stable
from popup import Editor,Capsule
E=Editor('1'*32,1,1,1,1,50,60,70,80,90)
def row(n,rect):
    return dict(hwnd=n,pid=50,tid=70,parent=0,owner=0,root=n,root_owner=n,xid=n+10,visible=1,enabled=1,minimized=0,dpi=96,
        style=0,exstyle=0,class_atom=7,class_hash=8,rect=rect,client=rect,focus=80,active=80,capture=0,pointer_hwnd=100)
def graph(rows,commit,qpc):
    xs={}
    for i,r in enumerate(rows):
        a,b,c,d=r['rect'];xs[str(r['xid'])]=dict(xid=r['xid'],rect=r['rect'],viewable=True,frame=r['xid'],parent_chain=[r['xid']],stack_index=i,
                input_shape=[[0,0,c-a,d-b]],bounding_shape=[[0,0,c-a,d-b]])
    return dict(editor=asdict(E),windows=rows,x11=xs,commit=commit,qpc=qpc,interval_ns=[qpc*1000,qpc*1000+500],terminal=False,complete=True)
def fixture(count=5):
    root=row(80,[0,0,800,600]);a=graph([root],1,100)
    rects=[[100,100,300,200],[100,90,300,100],[100,200,300,210],[90,90,100,210],[300,90,310,210]][:count]
    rows=[row(100+i,x) for i,x in enumerate(rects)];b=graph([root]+rows,2,200)
    for r in rows[1:]:b['x11'][str(r['xid'])]['input_shape']=[]
    events=[]
    for i,r in enumerate(rows):
        for j,source,message in ((0,9,3),(1,10,0x8002)):
            events.append(dict(action=1,hwnd=r['hwnd'],source=source,message=message,qpc=120+i*3+j,commit=i*2+j+1))
    receipt=dict(action=1,complete=True,x11=[dict(kind=k,action=1) for k in (4,5)],win32=[dict(kind=k,action=1,hwnd=80) for k in (0x201,0x202)])
    return a,receipt,b,events
class Tests(unittest.TestCase):
    def test_single_and_composite_no_fixed_count(self):
        for count in (1,2,3,4,5):
            a,r,b,l=fixture(count);g=Transaction(E,1,a,r,b,l).bind()
            self.assertEqual(len(g.members),count);self.assertEqual(g.target['hwnd'],100)
            self.assertEqual(g.target_at((150,150),b,b,110,100)['hwnd'],100)
            for point in ((95,95),(1000,1000)):
                with self.assertRaises(RuntimeError):g.target_at(point,b,b,110,100)
    def test_preexisting_same_thread_is_not_adopted(self):
        a,r,b,l=fixture();peer=row(999,[900,100,1000,200]);a=graph(a['windows']+[peer],1,100);b=graph(b['windows']+[peer],2,200)
        for w in b['windows'][2:6]:b['x11'][str(w['xid'])]['input_shape']=[]
        self.assertEqual(len(Transaction(E,1,a,r,b,l).bind().members),5)
    def test_new_separate_plausible_group_is_ambiguous(self):
        a,r,b,l=fixture(1);p=row(999,[900,100,1000,200]);b=graph(b['windows']+[p],2,200)
        l.extend([dict(action=1,hwnd=999,source=9,message=3,qpc=150,commit=20),dict(action=1,hwnd=999,source=10,message=0x8002,qpc=151,commit=21)])
        with self.assertRaisesRegex(RuntimeError,'ambiguous'):Transaction(E,1,a,r,b,l).bind()
    def test_exact_signal_refusals(self):
        for change in ('thread','process','owner','epoch','terminal','missing_show','recreate','missing_shape','stack_hole','two_inputs','wrong_receipt','late'):
            a,r,b,l=fixture()
            if change=='thread':b['windows'][1]['tid']=71
            if change=='process':b['windows'][1]['pid']=51
            if change=='owner':b['windows'][1]['owner']=999
            if change=='epoch':b['editor']['epoch']=2
            if change=='terminal':b['terminal']=True
            if change=='missing_show':l=[x for x in l if x['source']!=10]
            if change=='recreate':l.append(dict(l[0],commit=99))
            if change=='missing_shape':b['x11']['110']['input_shape']=None
            if change=='stack_hole':b['x11']['111']['stack_index']=20
            if change=='two_inputs':b['x11']['111']['input_shape']=[[0,0,200,10]]
            if change=='wrong_receipt':r['win32'][0]['hwnd']=999
            if change=='late':b['interval_ns'][1]=3_000_000_000
            with self.subTest(change=change),self.assertRaises(RuntimeError):Transaction(E,1,a,r,b,l).bind()
    def test_group_expiry_replacement_nested_and_pointer(self):
        a,r,b,l=fixture();g=Transaction(E,1,a,r,b,l).bind()
        for change in ('gone','xid','rect','shape','nested','focus','capture'):
            c=copy.deepcopy(b)
            if change=='gone':c['windows'].pop()
            if change=='xid':c['windows'][1]['xid']=999
            if change=='rect':c['windows'][1]['rect'][0]+=1
            if change=='shape':c['x11']['110']['input_shape']=[]
            if change=='nested':c['windows'].append(row(1000,[300,100,500,200]))
            if change=='focus':c['windows'][1]['focus']=777
            if change=='capture':c['windows'][1]['capture']=777
            with self.subTest(change=change),self.assertRaises(RuntimeError):g.revalidate(c,E)
        with self.assertRaises(RuntimeError):g.revalidate(b,replace(E,epoch=2))
        with self.assertRaises(RuntimeError):g.revalidate(b,E,[dict(action=2,hwnd=100,source=9,message=4,qpc=210)])
        for xid,hwnd in ((111,100),(110,101)):
            with self.assertRaises(RuntimeError):g.target_at((150,150),b,b,xid,hwnd)
    def test_change_while_down_releases_only_owned_input(self):
        from desktop import click_pair
        class Target:
            def __init__(self):self.calls=[]
            def button(self,down):self.calls.append(down);return down
        target=Target()
        def changed():raise RuntimeError('group replaced')
        with self.assertRaisesRegex(RuntimeError,'replaced'):click_pair(target,changed)
        self.assertEqual(target.calls,[True,False])
    def test_renderer_only_and_transparent_overlap(self):
        a,r,b,l=fixture();b['windows'][2]['rect']=[95,95,305,205]
        b['windows'][2]['client']=b['windows'][2]['rect'];b['x11']['111']['rect']=b['windows'][2]['rect']
        g=Transaction(E,1,a,r,b,l).bind()
        self.assertEqual(g.target_at((150,150),b,b,110,100)['hwnd'],100)
        from desktop import unobscured
        stack=[dict(xid=110,visible=True,rect=[100,100,300,200]),dict(xid=111,visible=True,rect=[95,95,305,205])]
        unobscured(stack,110,[100,100,300,200],{111})
        stack.append(dict(xid=999,visible=True,rect=[100,100,300,200]))
        with self.assertRaises(RuntimeError):unobscured(stack,110,[100,100,300,200],{111})
        b['x11']['110']['input_shape']=[]
        with self.assertRaisesRegex(RuntimeError,'absent'):Transaction(E,1,a,r,b,l).bind()
    def test_astra_needs_closed_custodian_reason(self):
        a,r,b,l=fixture();t=Transaction(E,1,a,r,b,l).bind().target
        c=Capsule('tsg',E,t,'resize_window',(150,150),1,1000)
        self.assertEqual(len(replace(c,executor='astra',escalation_reason='Resolve the newly observed nested choice').validate(E,2)),64)
        for reason in ('',' ','a'*257,'bad\nreason',None):
            with self.assertRaises(RuntimeError):replace(c,executor='astra',escalation_reason=reason).validate(E,2)
        with self.assertRaises(RuntimeError):replace(c,escalation_reason='not an escalation').validate(E,2)
if __name__=='__main__':unittest.main()

"""MF1 focus selection refuses unowned/ambiguous/recycled main identities.

No X server is opened for any rejected process identity.
"""
import ctypes,hashlib,pathlib,tempfile,unittest
from unittest.mock import patch
import session

class Scope:
    def __init__(self, members): self.records=members
    def members(self):return self.records

class OwnedScope(Scope):
    def __init__(self, pids):
        super().__init__([{'pid':pid,'start_ticks':pid*10} for pid in pids])
        self.current={r['pid']:dict(r) for r in self.records}
        self.groups={pid:'/owned' for pid in pids}
    def identity(self,pid):return self.current.get(pid)
    def group_of(self,pid):return self.groups.get(pid)
    def contains(self,group):return group=='/owned'

class Function:
    def __init__(self,run):self.run=run
    def __call__(self,*args):return self.run(*args)

class Desktop:
    """X11 property/event fixture, including the observed active window."""
    def __init__(self,windows,activate=True,direct_user_only=False):
        self.windows=windows;self.activate=activate;self.active=99;self.direct_user_only=direct_user_only
        self.atoms={};self.buffers=[];self.events=[];self.pid_reads={};self.changed_pid=None
        self.XOpenDisplay=Function(lambda *_:1);self.XCloseDisplay=Function(lambda *_:0)
        self.XDefaultRootWindow=Function(lambda *_:1);self.XSetErrorHandler=Function(lambda *_:0)
        self.XInternAtom=Function(self.atom);self.XGetWindowProperty=Function(self.property)
        self.XSync=Function(lambda *_:0);self.XFree=Function(lambda *_:0)
        self.XSendEvent=Function(self.send)
    def atom(self,display,name,only):
        if name not in self.atoms:self.atoms[name]=len(self.atoms)+1
        return self.atoms[name]
    def property(self,display,window,atom,offset,length,delete,requested,kind,fmt,count,after,data):
        name=next(k for k,v in self.atoms.items() if v==atom);values=[]
        if window==1 and name==b'_NET_CLIENT_LIST':values=list(self.windows)
        elif window==1 and name==b'_NET_ACTIVE_WINDOW':values=[self.active]
        elif window in self.windows and name==b'WM_STATE':values=[1,0]
        elif window in self.windows and name==b'_NET_WM_PID':
            self.pid_reads[window]=self.pid_reads.get(window,0)+1
            values=[self.changed_pid if self.changed_pid is not None and self.pid_reads[window]>1 else self.windows[window]]
        ctypes.cast(kind,ctypes.POINTER(ctypes.c_ulong))[0]=1 if values else 0
        ctypes.cast(fmt,ctypes.POINTER(ctypes.c_int))[0]=32
        ctypes.cast(count,ctypes.POINTER(ctypes.c_ulong))[0]=len(values)
        ctypes.cast(after,ctypes.POINTER(ctypes.c_ulong))[0]=0
        if values:
            buffer=(ctypes.c_ulong*len(values))(*values);self.buffers.append(buffer)
            ctypes.cast(data,ctypes.POINTER(ctypes.c_void_p))[0]=ctypes.addressof(buffer)
        return 0
    def send(self,display,root,propagate,mask,event):
        value=event._obj.client
        name=next(k for k,v in self.atoms.items() if v==value.message)
        self.events.append((name,value.window,value.data[0]))
        if name==b'_NET_ACTIVE_WINDOW' and self.activate and (not self.direct_user_only or value.data[0]==2):self.active=value.window
        return 1

class OperatorFocus(unittest.TestCase):
    def test_only_unique_registered_main_reaches_the_x11_boundary(self):
        app={'executable':{'path':'exact-main','sha256':'a'}}
        for records,images in [([],[]),([{'pid':1}],[]),([{'pid':1},{'pid':2}],[{'path':'exact-main','sha256':'a'}]),([{'pid':1}],[{'path':'exact-main','sha256':'different'}])]:
            with self.subTest(records=records,images=images),patch.object(session,'vendor_process_metadata',return_value={'registered_images':images}),patch.object(session.ctypes,'CDLL',side_effect=AssertionError('must not open X11')):
                with self.assertRaises(RuntimeError):session.vendor_focus(Scope(records),app)

    def control(self,scope,desktop,**kwargs):
        app=kwargs.pop('app',{'schema':3,'executable':{'path':'selected-program','sha256':'a'}})
        with patch.object(session.ctypes,'CDLL',return_value=desktop),patch.object(session,'vendor_process_metadata',side_effect=AssertionError('session control must not claim an image digest')):
            return session.vendor_focus(scope,app,**kwargs)

    def test_selected_session_controls_its_unique_helper_window(self):
        for close,expected in [(False,'focused'),(True,'close_requested')]:
            with self.subTest(close=close):
                desktop=Desktop({100:12,101:90})
                self.assertEqual(self.control(OwnedScope([11,12]),desktop,close=close),expected)
                self.assertEqual(desktop.events[0],(b'_NET_CLOSE_WINDOW' if close else b'_NET_ACTIVE_WINDOW',100,0 if close else 2))

    def test_explicit_focus_is_a_direct_user_action(self):
        desktop=Desktop({100:12,101:90},direct_user_only=True)
        with patch.object(session.time,'monotonic',side_effect=[0,1]):
            self.assertEqual(self.control(OwnedScope([11,12]),desktop),'focused')
        self.assertEqual(desktop.active,100)

    def test_session_control_refuses_unowned_ambiguous_or_changed_window_owners(self):
        for case in ['unowned','ambiguous','reused_pid','departed','changed_window_pid']:
            with self.subTest(case=case):
                scope=OwnedScope([11,12]);desktop=Desktop({100:12})
                if case=='unowned':desktop.windows={100:90}
                elif case=='ambiguous':desktop.windows[101]=11
                elif case=='reused_pid':scope.current[12]['start_ticks']+=1
                elif case=='departed':scope.groups[12]='/outside'
                else:desktop.changed_pid=90
                with self.assertRaises(RuntimeError):self.control(scope,desktop)
                self.assertEqual(desktop.events,[])

    def test_replaced_program_path_retains_exact_running_session_controls(self):
        with tempfile.TemporaryDirectory() as tmp:
            image=pathlib.Path(tmp).resolve()/'Program.exe';image.write_bytes(b'MZ build A')
            artifact={'path':str(image),'sha256':hashlib.sha256(image.read_bytes()).hexdigest()}
            held=session.RendererImage({'artifact':artifact,'size':image.stat().st_size})
            try:
                replacement=image.with_name('replacement');replacement.write_bytes(b'MZ build B');replacement.replace(image)
                for close,expected in [(False,'focused'),(True,'close_requested')]:
                    self.assertEqual(self.control(OwnedScope([11]),Desktop({100:11}),
                        app={'schema':3,'executable':artifact},close=close,selected_image=held),expected)
                with self.assertRaises(ValueError):held.check()
            finally:held.close()

    def test_window_manager_refusal_remains_distinct(self):
        with patch.object(session.time,'monotonic',side_effect=[0,1]):
            self.assertEqual(self.control(OwnedScope([11]),Desktop({100:11},activate=False)),'window_manager_refused')

    def test_legacy_adapter_still_requires_its_registered_main_image(self):
        app={'schema':2,'executable':{'path':'legacy-main','sha256':'a'}}
        desktop=Desktop({100:12});scope=OwnedScope([11,12])
        def metadata(scope,record,app,**kwargs):
            return {'registered_images':[app['executable']] if record['pid']==11 else []}
        with patch.object(session.ctypes,'CDLL',return_value=desktop),patch.object(session,'vendor_process_metadata',side_effect=metadata):
            with self.assertRaisesRegex(RuntimeError,'window absent'):session.vendor_focus(scope,app)
        self.assertEqual(desktop.events,[])

if __name__=='__main__':unittest.main()

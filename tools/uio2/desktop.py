"""Popup-local XWayland capture/input, using the existing XTEST owner.

X11 stacking is checked before and after private exact-drawable capture. No
whole-desktop image is read. Native Wayland-only surfaces are not supported.
"""
import ctypes as C
import time
from popup import bind
from x11 import X11, U, I, P

class Attributes(C.Structure):
    _fields_ = [('x', I), ('y', I), ('width', I), ('height', I), ('border', I),
                ('depth', I), ('visual', P), ('root', U), ('cls', I),
                ('bit_gravity', I), ('win_gravity', I), ('backing_store', I),
                ('backing_planes', U), ('backing_pixel', U), ('save_under', I),
                ('colormap', U), ('installed', I), ('map_state', I),
                ('all_events', C.c_long), ('your_events', C.c_long), ('do_not_propagate', C.c_long),
                ('override_redirect', I), ('screen', P)]

def overlaps(a, b):
    return a[0] < b[2] and a[2] > b[0] and a[1] < b[3] and a[3] > b[1]


def unobscured(stack, target, rect):
    matches = [i for i, row in enumerate(stack) if row['xid'] == target]
    if len(matches) != 1 or not stack[matches[0]]['visible']:
        raise RuntimeError('popup absent from mapped X11 stack')
    for row in stack[matches[0]+1:]:
        if row['visible'] and overlaps(row['rect'], rect):
            raise RuntimeError('foreign occlusion over popup')


class PopupX11(X11):
    def __init__(self, row, editor, fresh, current, terminal):
        self.row = row; self.editor = editor
        self.fresh = fresh; self.current = current; self.terminal = terminal
        # Wine _NET_WM_PID is a Unix PID, distinct from the observed Windows PID.
        # Authority is the exact Wine HWND->XID property plus process/start/thread.
        super().__init__(row['xid'])
        self.x.XQueryTree.argtypes=[P,U,C.POINTER(U),C.POINTER(U),C.POINTER(P),C.POINTER(C.c_uint)]
        self.x.XQueryTree.restype=I
        self.x.XGetWindowAttributes.argtypes=[P,U,C.POINTER(Attributes)]
        self.x.XGetWindowAttributes.restype=I
        try:self.guard()
        except BaseException:
            self.close();raise

    def tree(self, window):
        root=U();parent=U();children=P();count=C.c_uint()
        ok=self.x.XQueryTree(self.display,window,C.byref(root),C.byref(parent),C.byref(children),C.byref(count))
        self.sync()
        try:
            if not ok or count.value>2048:raise RuntimeError('X11 stack unavailable or exceeds bound')
            rows=list((U*count.value).from_address(children.value)) if children.value else []
            return parent.value,rows
        finally:
            if children.value:self.x.XFree(children)

    def stack(self):
        # A managed popup may have an X11 WM frame. Bind that frame by the exact
        # XID parent chain; a name or a matching rectangle is never sufficient.
        frame=self.window;seen=set()
        for _ in range(16):
            if frame in seen:raise RuntimeError('X11 parent cycle')
            seen.add(frame);parent,_=self.tree(frame)
            if parent==self.root:break
            frame=parent
        else:raise RuntimeError('X11 parent depth')
        _,children=self.tree(self.root);rows=[]
        for child in children:
            a=Attributes()
            if not self.x.XGetWindowAttributes(self.display,child,C.byref(a)):
                self.sync();raise RuntimeError('X11 stacking changed')
            self.sync()
            rows.append(dict(xid=child,visible=a.map_state==2,rect=[a.x,a.y,a.x+a.width,a.y+a.height]))
        return frame,rows

    def guard(self):
        if self.terminal():raise RuntimeError('terminal instance; stop GUI actions')
        bind(self.fresh(),self.editor,self.current(),self.row)
        rect=self.geometry()
        # The Wine whole-window property is in screen/client units at this
        # fixture. Refuse a scaled/offset guess if the two APIs disagree.
        if [rect[0],rect[1],rect[0]+rect[2],rect[1]+rect[3]] not in (self.row['rect'],self.row['client']):
            raise RuntimeError('Win32/XWayland popup geometry mismatch')
        a=Attributes()
        if not self.x.XGetWindowAttributes(self.display,self.window,C.byref(a)) or a.map_state!=2:
            self.sync();raise RuntimeError('popup no longer viewable')
        self.sync()
        active=self.property(self.root,'_NET_ACTIVE_WINDOW')
        if active not in ([self.editor.xid],[self.window]):raise RuntimeError('editor application not active')
        frame,rows=self.stack();unobscured(rows,frame,self.row['rect'])
        return dict(rect=list(rect),frame=frame,stack_count=len(rows),active=active,
                    sampled_ns=time.monotonic_ns(),scope='XWayland exact-window stack')

    def active_for_input(self, pointer):
        self.guard()
        return pointer['active'] in ([self.editor.xid],[self.window])

    def pointer_after_input(self, down):
        if down:return self.pointer()
        try:return self.pointer()
        except RuntimeError:
            # A successful menu action can destroy its popup synchronously on
            # Up. The event receipt must then come from X RECORD/Win32 hooks,
            # never from a guessed replacement target.
            snapshot=self.fresh()
            if any(r['hwnd']==self.row['hwnd'] for r in snapshot['windows']):raise
            return dict(unavailable=True,reason='selected_popup_absent_after_up')

    def capture_ready(self):self.guard()

    def capture(self):
        before=self.guard();meta,pixels=super().capture();after=self.guard()
        if before['rect']!=after['rect'] or before['frame']!=after['frame']:
            raise RuntimeError('popup changed during capture')
        meta['popup_before']=before;meta['popup_after']=after
        return meta,pixels

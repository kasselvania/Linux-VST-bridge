"""Popup-local XWayland capture/input, using the existing XTEST owner.

X11 stacking is checked before and after private exact-drawable capture. No
whole-desktop image is read. Native Wayland-only surfaces are not supported.
"""
import ctypes as C
import time
from popup import bind
from x11 import X11, U, I, P, Attributes

def overlaps(a, b):
    return a[0] < b[2] and a[2] > b[0] and a[1] < b[3] and a[3] > b[1]


def unobscured(stack, target, rect, transparent_members=()):
    matches = [i for i, row in enumerate(stack) if row['xid'] == target]
    if len(matches) != 1 or not stack[matches[0]]['visible']:
        raise RuntimeError('popup absent from mapped X11 stack')
    for row in stack[matches[0]+1:]:
        if row['visible'] and row['xid'] not in transparent_members and overlaps(row['rect'], rect):
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

class Rectangle(C.Structure):
    _fields_=[('x',C.c_short),('y',C.c_short),('width',C.c_ushort),('height',C.c_ushort)]

class SurfaceGraphX11(X11):
    """Read-only X graph/Shape collector; no window name or screenshot authority."""
    tree=PopupX11.tree
    stack=PopupX11.stack
    def __init__(self,window):
        super().__init__(window)
        import ctypes.util
        self.x.XQueryTree.argtypes=[P,U,C.POINTER(U),C.POINTER(U),C.POINTER(P),C.POINTER(C.c_uint)]
        self.x.XQueryTree.restype=I
        self.shape=C.CDLL(ctypes.util.find_library('Xext'))
        self.shape.XShapeQueryVersion.argtypes=[P,C.POINTER(I),C.POINTER(I)];self.shape.XShapeQueryVersion.restype=I
        self.shape.XShapeGetRectangles.argtypes=[P,U,I,C.POINTER(I),C.POINTER(I)]
        self.shape.XShapeGetRectangles.restype=C.POINTER(Rectangle)
        major=I();minor=I()
        if not self.shape.XShapeQueryVersion(self.display,C.byref(major),C.byref(minor)) or (major.value,minor.value)<(1,1):
            self.close();raise RuntimeError('X Shape 1.1 input region unavailable')
    def rectangles(self,window,kind):
        n=I();order=I();p=self.shape.XShapeGetRectangles(self.display,window,kind,C.byref(n),C.byref(order))
        try:
            self.sync()
            if not 0<=n.value<=256 or (n.value and not p):raise RuntimeError('X Shape extent/unavailable')
            return [[p[i].x,p[i].y,p[i].x+p[i].width,p[i].y+p[i].height] for i in range(n.value)]
        finally:
            if p:self.x.XFree(p)
    def facts(self,window):
        a=Attributes()
        if not self.x.XGetWindowAttributes(self.display,window,C.byref(a)):self.sync();raise RuntimeError('X surface vanished')
        self.sync();chain=[window]
        for _ in range(16):
            parent,_=self.tree(chain[-1])
            if parent==self.root:break
            if not parent or parent in chain:raise RuntimeError('X parent chain')
            chain.append(parent)
        else:raise RuntimeError('X parent chain bound')
        _,stack=self.tree(self.root)
        if chain[-1] not in stack:raise RuntimeError('X frame absent')
        px=I();py=I();child=U()
        if not self.x.XTranslateCoordinates(self.display,window,self.root,0,0,C.byref(px),C.byref(py),C.byref(child)):
            raise RuntimeError('X coordinates unavailable')
        self.sync()
        return dict(xid=window,frame=chain[-1],parent_chain=chain,stack_index=stack.index(chain[-1]),
                    rect=[px.value,py.value,px.value+a.width,py.value+a.height],viewable=a.map_state==2,
                    override_redirect=bool(a.override_redirect),transient_for=self.property(window,'WM_TRANSIENT_FOR'),
                    window_type=self.property(window,'_NET_WM_WINDOW_TYPE'),wm_state=self.property(window,'_NET_WM_STATE'),
                    net_wm_pid=self.property(window,'_NET_WM_PID'),
                    bounding_shape=self.rectangles(window,0),input_shape=self.rectangles(window,2))
    def enrich(self,graph,terminal=False):
        # Query every exactly mapped surface, including pre-action invisible
        # windows. Missing bindings remain absent, never invented.
        graph=dict(graph);graph['x11']={}
        for r in graph['windows']:
            if r['xid']:graph['x11'][str(r['xid'])]=self.facts(r['xid'])
        graph['terminal']=bool(terminal);graph['complete']=True
        graph['interval_ns'][1]=time.monotonic_ns()
        return graph
    def pointer_target(self):
        target=self.root
        for _ in range(32):
            root=U();child=U();rx=I();ry=I();wx=I();wy=I();mask=C.c_uint()
            if not self.x.XQueryPointer(self.display,target,C.byref(root),C.byref(child),C.byref(rx),C.byref(ry),C.byref(wx),C.byref(wy),C.byref(mask)):
                raise RuntimeError('pointer target unavailable')
            self.sync()
            if not child.value:return target
            target=child.value
        raise RuntimeError('pointer tree bound')

class GroupX11(PopupX11):
    """Existing input/capture owner with TSG1 replacing only popup authority."""
    def __init__(self,group,editor,fresh,current,terminal,lifecycle,xgraph):
        self.group=group;self.lifecycle=lifecycle;self.xgraph=xgraph
        super().__init__(group.target,editor,fresh,current,terminal)
    def guard(self):
        if self.terminal():raise RuntimeError('terminal instance; stop')
        g=self.fresh();self.group.revalidate(g,self.current(),self.lifecycle())
        geometry=self.geometry()
        if [geometry[0],geometry[1],geometry[0]+geometry[2],geometry[1]+geometry[3]] not in (self.row['rect'],self.row['client']):raise RuntimeError('group capture geometry')
        active=self.property(self.root,'_NET_ACTIVE_WINDOW');allowed=[self.editor.xid]+[r['xid'] for r in self.group.members]
        if len(active)!=1 or active[0] not in allowed:raise RuntimeError('group application not active')
        frame,stack=self.stack()
        # Capture is the exact content drawable, not a composited group image.
        # Only verified same-group input-transparent frames may overlap; foreign
        # surfaces always refuse. Their pixels are not called content pixels.
        transparent={g['x11'][str(r['xid'])]['frame'] for r in self.group.members
                     if not g['x11'][str(r['xid'])]['input_shape']}
        unobscured(stack,frame,self.row['rect'],transparent)
        return dict(rect=list(geometry),frame=frame,stack_count=len(stack),active=active,sampled_ns=time.monotonic_ns(),group=self.group.identity)
    def active_for_input(self,pointer):
        self.guard();return pointer['active'] in [[self.editor.xid]]+[[r['xid']] for r in self.group.members]
    def verify_point(self,point):
        first=self.fresh();second=self.fresh();target=self.xgraph.pointer_target()
        # Wine may expose an input child below the whole-window XID. Resolve
        # only its exact server parent chain, not coordinates or a PID guess.
        for _ in range(16):
            if target==self.row['xid']:break
            target,_=self.tree(target)
            if target==self.root:break
        root=next(r for r in second['windows'] if r['hwnd']==self.row['hwnd'])
        return self.group.target_at(point,first,second,target,root['pointer_hwnd'])

def click_pair(target, while_down):
    """The only owned button is released even if an identity check fails mid-click.
    A failure never authorizes a replacement target, retry or another Down.
    """
    down=None
    try:
        down=target.button(True);while_down()
    finally:
        if down is not None or 1 in getattr(target,'buttons',set()):target_up=target.button(False)
    return down,target_up

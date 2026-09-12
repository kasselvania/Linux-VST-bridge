"""Verify Win32 injected-input observation on the pinned runner, not Pigments.
The generated fixture owns its temporary blank windows. Restore the previously
active exact editor after the fixture closes; no vendor control is clicked.
"""
import json
import os
import pathlib
import sys
from launch import Context
from capture import selected_window
from x11 import X11

if __name__=='__main__':
    c=Context(pathlib.Path(sys.argv[1]),sys.argv[2],pathlib.Path(__file__).parent/'package')
    out=c.package/'runner-fixture';out.mkdir(mode=0o700)
    process,rows,snapshot=c.census(out/'census.log');_,xid=selected_window(rows)
    for k in ('DISPLAY','XAUTHORITY'):
        if k in c.env:os.environ[k]=c.env[k]
    with X11(xid) as x:
        before=x.pointer();result=c.helper('uio1-tests.exe',[],out/'fixture.log',15).finish()
        if before['active']==[xid]:x.activate()
        rect=x.geometry();x.move((before['client'][0]/(rect[2]-1),before['client'][1]/(rect[3]-1)))
        try:x.settle_pointer();pointer_restored=True
        except RuntimeError:pointer_restored=False # no click is sent on failed warp
    c.fault.close()
    print(json.dumps(dict(fixture=result,original_editor_restored=True,pointer_restored=pointer_restored)))

"""Optional, bounded UIA census after the primary interaction record.
Never reads Name, Value, text patterns, or invokes controls. Provider timeout
retires only the separate diagnostic helper; the vendor host is not killed.
"""
import json
import pathlib
import sys
from launch import Context,private_json
from capture import selected_window
from observe import GuiWitness

if __name__=='__main__':
    c=Context(pathlib.Path(sys.argv[1]),sys.argv[2],pathlib.Path(__file__).parent/'package')
    out=c.package/'accessibility';out.mkdir(mode=0o700)
    process,rows,snapshot=c.census(out/'census.log');w,_=selected_window(rows)
    gui=GuiWitness(pathlib.Path(c.session['directory'])/'ap11.ui',c.session['session'])
    gui.cursors[1]=max(0,gui.cursors[1]-512)
    history=[r for r in gui.take() if r['kind']==108];gui.close()
    private_json(out/'logical-editor-status.json',history)
    try:result=c.helper('uio1-accessibility.exe',[w['hwnd']],out/'accessibility.log',3).finish()
    except TimeoutError:result={'timed_out':True}
    records=[json.loads(s) for s in (out/'accessibility.log').read_text().splitlines() if s.startswith('{')]
    private_json(out/'result.json',dict(helper=result,records=records))
    c.fault.close()
    print(json.dumps(dict(helper=result,node_count=len(records),logical_status_count=len(history))))

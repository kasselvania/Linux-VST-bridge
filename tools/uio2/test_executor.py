from dataclasses import asdict
import http.client
import json
from pathlib import Path
import tempfile
import threading
import time
import unittest
from unittest.mock import Mock
from test_popup import E,graph
from popup import Capsule,bind
from executor import Surface,png

class Tests(unittest.TestCase):
    def test_closed_http_surface_cannot_inject_coordinates_or_repeat(self):
        with tempfile.TemporaryDirectory() as d:
            out=Path(d);(out/'capsules').mkdir()
            now=time.monotonic_ns();c=Capsule('surface-test',E,bind(graph(),E,E),'resize_window',(200,150),now,now+5_000_000_000)
            p=out/'capsules/1.json';p.write_text(json.dumps(asdict(c)));p.chmod(0o600)
            owner=Mock();owner.current.return_value=E;owner.act.return_value={'input_sent':True}
            s=Surface(owner,out);t=threading.Thread(target=s.http.serve_forever,daemon=True);t.start()
            def post(body,origin=True):
                conn=http.client.HTTPConnection(s.host,timeout=2)
                headers={'Content-Type':'application/json'}
                if origin:headers['Origin']='http://'+s.host
                conn.request('POST','/'+s.token+'/run',json.dumps(body),headers)
                r=conn.getresponse();b=r.read();conn.close();return r.status,b
            try:
                self.assertEqual(post({'action':'resize_window'},False)[0],403);owner.act.assert_not_called()
                code,body=post({'action':'resize_window'});self.assertEqual(code,200);self.assertTrue(json.loads(body)['custodian_must_interpret'])
                self.assertEqual(post({'action':'resize_window','point':[0,0]})[0],409)
                owner.act.assert_called_once();self.assertTrue(s.stopped)
                self.assertEqual(post({'action':'resize_window'})[0],409);owner.act.assert_called_once()
            finally:s.http.shutdown();t.join();s.close()
    def test_png_is_exact_bounded_private_conversion(self):
        import zlib,struct
        data=png(dict(width=2,height=1,stride=8),bytes([3,2,1,0,6,5,4,0]))
        self.assertTrue(data.startswith(b'\x89PNG'))
        at=data.index(b'IDAT');n=struct.unpack_from('>I',data,at-4)[0]
        self.assertEqual(zlib.decompress(data[at+4:at+4+n]),bytes([0,1,2,3,4,5,6]))
        with self.assertRaises(RuntimeError):png(dict(width=4097,height=1,stride=16388),b'')
    def test_empty_result_status_is_not_terminal_failure(self):
        from product import Product
        from types import SimpleNamespace
        p=Product.__new__(Product)
        s={'terminal_instance':None,'result_status':{'available':True,'rejection':None},'editor':{'failure':0,'closed':0}}
        p.c=SimpleNamespace(fault=SimpleNamespace(snapshot=lambda:s))
        self.assertFalse(p.terminal())
        s['result_status']['rejection']={'reason':'EventChannel'};self.assertTrue(p.terminal())

    def test_unowned_popup_never_falls_back_to_underlying_editor_frame(self):
        from product import Product
        from copy import deepcopy
        p=Product.__new__(Product);p.e=E;p.x=Mock();p.current=Mock(return_value=E)
        g=graph();root=next(r for r in g['windows'] if r['hwnd']==E.hwnd)
        peer=next(r for r in g['windows'] if r['hwnd']!=E.hwnd)
        g['windows']=[root]
        self.assertEqual(p.target(g),(root,p.x))
        # Observed Pigments shape: main menu plus four surrounding top levels,
        # same PID/thread, but GW_OWNER=0 and GA_ROOTOWNER=self for all five.
        for i in range(5):
            row=deepcopy(peer);row.update(hwnd=100+i,root=100+i,root_owner=100+i,owner=0,xid=200+i)
            g['windows'].append(row)
        p.fresh=Mock(return_value=g)
        with self.assertRaisesRegex(RuntimeError,'owned popup absent or ambiguous'):p.frame()
        p.x.capture.assert_not_called()

if __name__=='__main__':unittest.main()

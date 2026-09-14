"""A loopback-only, one-button surface for a custodian's frozen GUI capsule.

The executor can inspect an exact private frame and consume the next capsule.
It cannot supply a coordinate, path, command, retry or technical verdict. The
custodian writes capsules separately; this server never grants test authority.
"""
from dataclasses import asdict
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
import os
from pathlib import Path
import secrets
import stat
import struct
import time
import zlib
from popup import Capsule, Editor, Permit


def png(meta, bgra):
    w,h,stride=meta['width'],meta['height'],meta['stride']
    if not 1<=w<=4096 or not 1<=h<=4096 or stride<w*4 or len(bgra)!=stride*h:
        raise RuntimeError('private frame extent')
    raw=bytearray()
    for y in range(h):
        raw.append(0)
        for x in range(w):
            b,g,r,_=bgra[y*stride+x*4:y*stride+x*4+4];raw.extend((r,g,b))
    def chunk(t,b):return struct.pack('>I',len(b))+t+b+struct.pack('>I',zlib.crc32(t+b)&0xffffffff)
    return b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',w,h,8,2,0,0,0))+chunk(b'IDAT',zlib.compress(raw,3))+chunk(b'IEND',b'')


def read_capsule(path):
    fd=os.open(path,os.O_RDONLY|os.O_NOFOLLOW)
    with os.fdopen(fd,'rb') as f:
        s=os.fstat(f.fileno())
        if not stat.S_ISREG(s.st_mode) or s.st_uid!=os.getuid() or s.st_mode&0o077 or s.st_size>8192:
            raise RuntimeError('private capsule ownership/extent')
        data=f.read(8193)
    d=json.loads(data);d['editor']=Editor(**d['editor']);d['point']=tuple(d['point'])
    for key in ('forbidden','stop'):d[key]=tuple(d[key])
    return Capsule(**d)

class Surface:
    def __init__(self, owner, directory, port=0):
        self.owner=owner;self.directory=Path(directory);self.next=1;self.permit=None
        self.token=secrets.token_urlsafe(32);self.receipts=[];self.frames=0;self.stopped=False
        surface=self
        class Handler(BaseHTTPRequestHandler):
            def log_message(self,*args):pass # no URLs, private tokens or UI text in logs
            def send(self,code,kind,data):
                self.send_response(code);self.send_header('Content-Type',kind)
                self.send_header('Cache-Control','no-store');self.send_header('X-Content-Type-Options','nosniff')
                self.send_header('Content-Security-Policy',"default-src 'none'; img-src 'self'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; connect-src 'self'")
                self.send_header('Content-Length',str(len(data)));self.end_headers();self.wfile.write(data)
            def allowed(self):
                return self.client_address[0]=='127.0.0.1' and self.headers.get('Host')==surface.host
            def do_GET(self):
                if not self.allowed():return self.send(403,'text/plain',b'Forbidden')
                if self.path=='/'+surface.token:
                    html='''<!doctype html><meta charset="utf-8"><title>UIO2 closed action capsule</title>
<style>body{font:16px sans-serif;background:#222;color:white}img{max-width:100%;height:auto}pre{white-space:pre-wrap}</style>
<p>Private exact-window view. No action is authorized until the custodian supplies a capsule.</p>
<button id="run" disabled>Execute the one authorized action</button><pre id="status"></pre><img id="frame">
<script>
const base=location.pathname;let action=null;
async function load(){let r=await fetch(base+'/status');let s=await r.json();document.getElementById('status').textContent=JSON.stringify(s,null,2);action=s.action;document.getElementById('run').disabled=!action;document.getElementById('frame').src=base+'/frame';}
document.getElementById('run').onclick=async()=>{document.getElementById('run').disabled=true;let r=await fetch(base+'/run',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({action})});document.getElementById('status').textContent=await r.text();};load();
</script>'''
                    return self.send(200,'text/html',html.encode())
                try:
                    if self.path=='/'+surface.token+'/status':
                        return self.send(200,'application/json',json.dumps(surface.status()).encode())
                    if self.path=='/'+surface.token+'/frame':
                        if surface.frames>=24:raise RuntimeError('private frame capacity')
                        meta,pixels=surface.owner.frame();surface.frames+=1
                        return self.send(200,'image/png',png(meta,pixels))
                except Exception as e:
                    surface.stop(type(e).__name__+": "+str(e))
                    return self.send(409,'application/json',b'{"stopped":true,"detail":"custodian inspection required"}')
                self.send(404,'text/plain',b'Not found')
            def do_POST(self):
                if (not self.allowed() or self.path!='/'+surface.token+'/run' or
                    self.headers.get('Origin')!='http://'+surface.host or self.headers.get('Content-Type')!='application/json'):
                    return self.send(403,'text/plain',b'Forbidden')
                try:
                    length=int(self.headers.get('Content-Length','0'))
                    if not 1<=length<=128:raise RuntimeError('command extent')
                    receipt=surface.execute(json.loads(self.rfile.read(length)))
                    self.send(200,'application/json',json.dumps(receipt).encode())
                except Exception as e:
                    surface.stop(type(e).__name__+": "+str(e))
                    self.send(409,'application/json',b'{"stopped":true,"detail":"custodian inspection required"}')
        self.http=HTTPServer(('127.0.0.1',port),Handler);self.http.timeout=.02
        self.host='127.0.0.1:'+str(self.http.server_port)

    def stop(self,reason):
        self.stopped=True;self.owner.stop(reason)

    def status(self):
        if self.stopped:return dict(stopped=True)
        self.owner.current()
        path=self.directory/'capsules'/f'{self.next}.json'
        if self.permit is None and path.exists():
            if self.next>6:raise RuntimeError('action capacity')
            self.permit=Permit(read_capsule(path))
        if not self.permit:return dict(action=None,waiting_for_custodian=True)
        c=self.permit.capsule;c.validate(self.owner.current(),time.monotonic_ns())
        return dict(action=c.action,test_id=c.test_id,maximum_actions=1,executor=c.executor,escalation_reason=c.escalation_reason,group_identity=c.group_identity,
                    forbidden=c.forbidden,stop=c.stop,point=c.point,window=c.target['rect'])

    def execute(self,command):
        if self.stopped:raise RuntimeError('session stopped')
        self.status()
        if self.permit is None:raise RuntimeError('no custodian capsule')
        c=self.permit.capsule;fingerprint=self.permit.consume(command,self.owner.current(),time.monotonic_ns())
        result=self.owner.act(c,self.next)
        receipt=dict(capsule_sha256=fingerprint,action=c.action,attempted=True,machine=result)
        self.receipts.append(receipt);self.next+=1;self.permit=None
        return dict(action=c.action,attempted=True,custodian_must_interpret=True)

    def close(self):self.http.server_close()

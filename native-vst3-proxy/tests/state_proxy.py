#!/usr/bin/env python3
"""AP4 actual SDK bundle with an explicitly substituted Windows peer; zero live batches."""
import hashlib,json,mmap,os,pathlib,socket,struct,subprocess,sys,tempfile,threading,time
from loaded_proxy import exact

def frame(k,session,seq,p=b''):
 return struct.pack('<IHHHHI16sQQQ',0x3141504c,1,4,k,0,len(p),session,1,seq,0)+p

def receive(s):
 h=exact(s,56);magic,major,minor,k,flags,n,session,instance,seq,parent=struct.unpack('<IHHHHI16sQQQ',h)
 assert (magic,major,minor,flags,instance,parent)==(0x3141504c,1,4,0,1,0) and n<=(1<<20)
 return k,session,seq,exact(s,n)

def envelope(gain=.25,reduction=0.,bypass=0):
 p=struct.pack('<ffi',gain,reduction,bypass)
 return b'LVBSTATE'+struct.pack('<II',1,104)+bytes.fromhex('84e8de5f92554f5396fae4133c935a18')+bytes.fromhex('60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f')+struct.pack('<II',len(p),0)+hashlib.sha256(p).digest()+p

def peer(directory,case,counts,*,fail_after=None,close_delay=0,audio_hook=None):
 until=time.monotonic()+10
 while not (directory/'ap1.control').exists():
  if time.monotonic()>until:raise TimeoutError('endpoint not created')
  time.sleep(.002)
 raw=(directory/'ap1.control').read_bytes();port=struct.unpack_from('<H',raw)[0];session=raw[4:20]
 with (directory/'ap1.audio').open('r+b') as f,mmap.mmap(f.fileno(),4192) as m,socket.create_connection(('127.0.0.1',port),5) as s:
  s.settimeout(25);w=struct.unpack_from('<Q',m,32)[0]^0x8d396b274e105ac3;struct.pack_into('<Q',m,40,w)
  s.sendall(frame(1,session,0,raw[20:]+struct.pack('<II',256,4192)));assert receive(s)==(1,session,0,b'')
  seq=1;epoch=position=0;running=active=False;gain=1.;reduction=0.;bypass=0
  while True:
   try:k,token,number,p=receive(s)
   except (EOFError,ConnectionResetError):
    if case=='state-corrupt-output':return
    raise
   assert token==session and number==seq
   if k in (16,18):
    counts['get' if k==16 else 'set']+=1
    if k==18:assert not running;gain,reduction,bypass=struct.unpack('<ffi',p)
    else:assert not p
    payload=struct.pack('<ffi',gain,reduction,bypass)
    if case=='state-error':s.sendall(frame(7,session,seq,b'\x01\x00\x00\x00'))
    elif case=='state-stale':s.sendall(frame(k+1,session,seq+1,payload))
    elif case=='state-lost-set' and k==18:s.shutdown(socket.SHUT_WR)
    elif case=='recovery-bad-readback' and k==18:s.sendall(frame(k+1,session,seq,bytes([payload[0]^1])+payload[1:]))
    else:
     # Delay a coherent save less than the retained audio pipeline latency.
     if running:time.sleep(.001)
     s.sendall(frame(k+1,session,seq,payload));seq+=1;continue
    try:assert s.recv(1)==b'' # no retransmission after ambiguous response
    except ConnectionResetError:pass
    return
   if k==8:assert not active and p==struct.pack('<II',256,0);active=True;counts['activate']+=1;s.sendall(frame(9,session,seq));continue
   if k==10:assert active and not running and p==struct.pack('<Q',epoch+1);epoch+=1;position=0;running=True;counts['start']+=1;s.sendall(frame(11,session,seq,p));continue
   if k==12:assert running and p==struct.pack('<Q',epoch);running=False;counts['stop']+=1;s.sendall(frame(13,session,seq,p));continue
   if k==14:assert active and not running and not p;active=False;s.sendall(frame(15,session,seq));continue
   if k==5:assert not active and not running and not p;time.sleep(close_delay);s.sendall(frame(6,session,seq));counts['closed']+=1;return
   assert k==3 and running and len(p)==48
   n,ino,outo,stride,g,flags,present,e,pos=struct.unpack('<IIIIdIIQQ',p)
   assert (ino,outo,stride,e,pos)==(64,2128,1032,epoch,position) and present in (0,1)
   counts['process']+=1;counts['zero_frame']+=n==0
   if fail_after and counts['process']>=fail_after:return
   if audio_hook:audio_hook(position,n,present,g)
   if case=='state-corrupt-output':struct.pack_into('<I',m,outo,0)
   if present:gain=struct.unpack('<f',struct.pack('<f',g))[0]
   factor=1 if bypass else max(0,gain-reduction)
   for ch in range(2):
    values=struct.unpack_from('<'+str(n)+'f',m,ino+ch*stride+4);struct.pack_into('<'+str(n)+'f',m,outo+ch*stride+4,*[v*factor for v in values])
   silence=3 if factor==0 or flags==3 else 0
   s.sendall(frame(4,session,seq,struct.pack('<IIQQQ',n,2128,silence,epoch,position)));seq+=1;position+=n

def run(host,bundle,audit):
 results=[]
 with tempfile.TemporaryDirectory(prefix='ap4-state-tests-') as tmp:
  root=pathlib.Path(tmp);store=root/'state';store.mkdir()
  for case in ('state-capture','state-gain','state-mute','state-error','state-stale','state-lost-set','state-corrupt-output'):
   directory=root/case;directory.mkdir();counts=dict(get=0,set=0,activate=0,start=0,stop=0,process=0,zero_frame=0,closed=0);errors=[]
   env={**os.environ,'LVB_AP2_SESSION_DIR':str(directory),'LVB_AP2_SESSION':os.urandom(16).hex(),'LVB_AP4_STATE_STORE':str(store),'LVB_AP4_COMPARE':'1','LD_PRELOAD':audit,'LVB_AP3_REPORT':str(directory/'proxy.jsonl')}
   def target():
    try:peer(directory,case,counts)
    except BaseException as e:errors.append(type(e).__name__+': '+str(e))
   t=threading.Thread(target=target);t.start();p=subprocess.run([host,bundle,case],env=env,capture_output=True,text=True,timeout=45);t.join(25)
   assert not t.is_alive() and not errors,(case,errors,p.stdout,p.stderr)
   failed=case in {'state-error','state-stale','state-lost-set','state-corrupt-output'}
   assert p.returncode==(1 if failed else 0),(case,p.returncode,p.stdout,p.stderr)
   records=[json.loads(l) for l in p.stdout.splitlines()]
   if case=='state-corrupt-output':
    faults=[r for r in records if r['event']=='ap4_native_error']
    assert len(faults)==1 and faults[0]['fault']==3,(case,records)
    assert faults[0]['stage']=='failed_instance' and faults[0]['callback_rejections']>0
    assert counts['process']==1 and counts['closed']==0
    retained=[json.loads(l) for l in (directory/'proxy.jsonl').read_text().splitlines()]
    assert faults[0] in retained
   elif failed:
    faults=[r for r in records if r['event']=='ap4_native_error'];assert len(faults)==1 and faults[0]['detail'],(case,records)
    assert counts['process']==0 and counts['closed']==0
    assert counts['set']==(1 if case=='state-lost-set' else 0)
    assert counts['get']==1
   else:
    assert counts['closed']==1 and counts['get']>=3
    compared=[r for r in records if r['event']=='ap4_host_compared'];assert compared and all(r['max_error']==0 and r['callback_effects']==0 for r in compared)
    if case not in {'state-local','state-capture'}:assert compared[0]['initial_gain_sent'] is False
   results.append(dict(case=case,counts=counts,records=records,stderr=p.stderr[-1500:]))
  assert (store/'gain.state').read_bytes()==envelope(.25)
  assert (store/'mute.state').read_bytes()==envelope(0.)
 print(json.dumps(dict(classification='LOCAL_SUBSTITUTED_PEER_TESTS',windows_workloads=0,results=results)))
if __name__=='__main__':run(*sys.argv[1:])

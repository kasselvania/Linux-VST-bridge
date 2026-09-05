#!/usr/bin/env python3
"""Substituted peer tests through the actual loaded AP3 VST3 bundle; no Windows."""
import json,mmap,os,pathlib,socket,struct,subprocess,tempfile,threading,time,sys
from loaded_proxy import exact

def frame(k,session,seq,p=b''):
 return struct.pack('<IHHHHI16sQQQ',0x3141504c,1,3,k,0,len(p),session,1,seq,0)+p

def receive(s):
 h=exact(s,56);magic,major,minor,k,flags,n,session,instance,seq,parent=struct.unpack('<IHHHHI16sQQQ',h)
 assert (magic,major,minor,flags,instance,parent)==(0x3141504c,1,3,0,1,0) and n<=4040
 return k,session,seq,exact(s,n)

def peer(directory,scenario,counts):
 until=time.monotonic()+10
 while not (directory/'ap1.control').exists():
  if time.monotonic()>until:raise TimeoutError('endpoint not created')
  time.sleep(.002)
 raw=(directory/'ap1.control').read_bytes();port=struct.unpack_from('<H',raw)[0];session=raw[4:20]
 with (directory/'ap1.audio').open('r+b') as f,mmap.mmap(f.fileno(),4192) as m,socket.create_connection(('127.0.0.1',port),5) as s:
  s.settimeout(15);w=struct.unpack_from('<Q',m,32)[0]^0x8d396b274e105ac3;struct.pack_into('<Q',m,40,w)
  s.sendall(frame(1,session,0,raw[20:]+struct.pack('<II',256,4192)));assert receive(s)==(1,session,0,b'');assert struct.unpack_from('<Q',m,56)[0]==w^1
  assert receive(s)==(8,session,1,struct.pack('<II',256,0));s.sendall(frame(9,session,1));counts['activate']+=1
  seq=1;epoch=0;position=0;running=False
  while True:
   k,token,number,p=receive(s);assert token==session and number==seq
   if k==10:
    assert not running and p==struct.pack('<Q',epoch+1);epoch+=1;position=0;running=True;counts['start']+=1
    if scenario=='overflow':
     time.sleep(.1)
     try:assert s.recv(1)==b''
     except ConnectionResetError:pass
     return
    s.sendall(frame(11,session,seq,p));continue
   if k==12:
    assert running and p==struct.pack('<Q',epoch);running=False;s.sendall(frame(13,session,seq,p));counts['stop']+=1;continue
   if k==14:
    assert not running and not p;s.sendall(frame(15,session,seq));assert receive(s)==(5,session,seq,b'');s.sendall(frame(6,session,seq));counts['closed']+=1;return
   assert k==3 and running and len(p)==48
   n,ino,outo,stride,gain,flags,reserved,e,pos=struct.unpack('<IIIIdIIQQ',p)
   assert (ino,outo,stride,reserved,e,pos)==(64,2128,1032,0,epoch,position)
   counts['process']+=1
   fault=scenario not in {'positive','reopen','delayed','core'}
   if fault and scenario=='disconnect':return
   if fault and scenario in {'timeout','underflow'}:time.sleep(.1 if scenario=='underflow' else 6)
   if scenario=='delayed':time.sleep(.0002)
   for ch in range(2):
    inp=struct.unpack_from('<'+str(n)+'f',m,64+ch*1032+4)
    out=[v*gain for v in inp]
    if fault and scenario=='nonfinite':out[0]=float('nan')
    struct.pack_into('<'+str(n)+'f',m,2128+ch*1032+4,*out)
   result_flags=3 if flags==3 or gain==0 else 0
   if fault and scenario=='flags':result_flags=1<<40
   if fault and scenario=='silence':result_flags=3
   reply=struct.pack('<IIQQQ',n,2128,result_flags,epoch+(scenario=='stale'),position+(scenario=='position'))
   try:s.sendall(frame(4,session,seq+(scenario=='sequence'),reply))
   except (BrokenPipeError,ConnectionResetError):return
   if fault:
    try:assert s.recv(1)==b'' # no retry/reconnect/new work after failure
    except ConnectionResetError:pass
    return
   seq+=1;position+=n

def run(host,bundle,audit):
 results=[]
 for case in ['positive','reopen','core','delayed','disconnect','timeout','underflow','overflow','stale','position','sequence','flags','silence','nonfinite']:
  with tempfile.TemporaryDirectory(prefix='ap3-local-') as temp:
   directory=pathlib.Path(temp);counts=dict(activate=0,start=0,stop=0,process=0,closed=0);errors=[]
   env={**os.environ,'LVB_AP2_SESSION_DIR':str(directory),'LVB_AP2_SESSION':os.urandom(16).hex(),'LD_PRELOAD':audit,'LVB_AP3_REPORT':str(directory/'proxy-report.jsonl')}
   def target():
    try:peer(directory,case,counts)
    except BaseException as error:errors.append(type(error).__name__+': '+str(error))
   t=threading.Thread(target=target);t.start()
   p=subprocess.run([host,bundle,'positive' if case=='delayed' else case],env=env,capture_output=True,text=True,timeout=85 if case=='core' else 35);t.join(16)
   assert not t.is_alive() and not errors,(case,errors)
   assert p.returncode==0,(case,p.returncode,p.stdout,p.stderr)
   report=(directory/'proxy-report.jsonl').read_bytes();assert 0<len(report)<=8192
   retained=[json.loads(line) for line in report.splitlines()]
   lifecycle=[r for r in retained if r['event']=='ap3_proxy_lifecycle'];assert len(lifecycle)==1
   assert lifecycle[0]['requested_rate']==48000 and lifecycle[0]['requested_maximum']==256 and lifecycle[0]['requested_mode']==0
   # The overflow fixture fails in setProcessing before calling process.
   assert (lifecycle[0]['callback_rejections']==0)==(case in {'positive','reopen','delayed','core','overflow'}),(case,lifecycle)
   assert lifecycle[0]['clean']==(case in {'positive','reopen','delayed','core'})
   assert any(r['event']=='ap3_proxy_stats' for r in retained)
   assert all(r in [json.loads(line) for line in p.stdout.splitlines()] for r in retained)
   if case not in {'positive','reopen','delayed','core'}:assert counts['process']==(0 if case=='overflow' else 1) and counts['closed']==0
   else:assert counts['activate']==1 and counts['start']==(1 if case=='reopen' else 2) and counts['closed']==1
   results.append(dict(case=case,peer_counts=counts,host_records=[json.loads(l) for l in p.stdout.splitlines()]))
 print(json.dumps(dict(classification='LOCAL_SUBSTITUTED_PEER_TESTS',windows_workloads=0,results=results)))
if __name__=='__main__':run(*sys.argv[1:])

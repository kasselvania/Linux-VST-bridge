#!/usr/bin/env python3
"""Actual SDK callback/worker, deterministic substituted peer; no Windows claim."""
import json, os, pathlib, shutil, socket, struct, subprocess, sys, tempfile, threading, time
from state_proxy import peer, envelope

def run(host, bundle, audit):
 results=[]
 for case in ('playback-partial','playback-expired'):
  with tempfile.TemporaryDirectory(prefix='ap7-playback-') as tmp:
   home=pathlib.Path(tmp); root=home/'AP4-State-Test/preview';root.mkdir(parents=True,mode=0o700)
   output=root/'results';output.mkdir(mode=0o700)
   store=home/'state';store.mkdir()
   (store/'instance-a.state').write_bytes(envelope(.25,.125));(store/'instance-b.state').write_bytes(envelope(.75))
   errors=[];workers=[];counts=[]
   with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as listener:
    address=root/'owner.sock';listener.bind(str(address));address.chmod(0o600);listener.listen(2);listener.settimeout(10)
    def serve(lease,index):
     try:
      directory=home/str(index);directory.mkdir(mode=0o700)
      counter=dict(get=0,set=0,activate=0,start=0,stop=0,process=0,zero_frame=0,closed=0,edits=0);counts.append(counter)
      def hook(position,n,present,gain):
       if index: return
       if position==0:
        assert not (store/'release-a').exists()
        (store/'a-held').write_text('held')
        until=time.monotonic()+3
        while not (store/'release-a').exists():
         if time.monotonic()>until:raise TimeoutError('driver did not release delayed output')
         time.sleep(.001)
       if present:counter['edits']+=1;assert gain==.5 and position==4096
       # This marks admission, not reply publication; the next due result
       # belongs to a request at least 1024 source frames before this one.
       pending=store/'a-through.tmp';pending.write_text(str(position+n));pending.replace(store/'a-through')
      with lease:
       lease.settimeout(20);assert lease.recv(4)==b'AP4\n'
       token=os.urandom(16).hex();response=(token+'\n'+str(directory)).encode()
       lease.sendall(struct.pack('<H',len(response))+response)
       peer(directory,'playback-peer',counter,audio_hook=hook)
       assert lease.recv(1)==b'';shutil.rmtree(directory);lease.sendall(b'R')
     except BaseException as error:errors.append((index,repr(error)))
    def accept():
     try:
      for index in range(2):
       lease,_=listener.accept();w=threading.Thread(target=serve,args=(lease,index));workers.append(w);w.start()
     except BaseException as error:errors.append(('accept',repr(error)))
    t=threading.Thread(target=accept);t.start()
    env={k:v for k,v in os.environ.items() if not k.startswith('LVB_')};env.update(HOME=str(home),LD_PRELOAD=audit,LVB_AP4_STATE_STORE=str(store))
    process=subprocess.run([host,bundle,case],env=env,capture_output=True,text=True,timeout=25)
    t.join(11)
    for w in workers:w.join(20)
    assert not t.is_alive() and all(not w.is_alive() for w in workers)
    assert process.returncode==0 and not errors,(case,errors,process.stdout,process.stderr)
    records=[json.loads(line) for path in output.glob('native-*.jsonl') for line in path.read_text().splitlines()]
    lifecycle=[r for r in records if r['event']=='ap3_proxy_lifecycle'];assert len(lifecycle)==2
    a=next(r for r in lifecycle if r['underrun_frames']);b=next(r for r in lifecycle if not r['underrun_frames'])
    assert a['underrun_frames']==(128 if case=='playback-partial' else 768) and a['underrun_gaps']==1
    assert a['expired_output_frames']==a['underrun_frames'] and a['priming_frames']==1024
    assert all(r['callback_rejections']==r['discontinuities']==0 for r in lifecycle)
    assert all(c['start']==c['activate']==c['set']==c['closed']==1 for c in counts)
    assert counts[0]['edits']==1
    observations=[r for r in records if r['event']=='ap7_observation'];assert len(observations)==2
    assert all(r['offered_samples']==r['verified_returned_samples'] and r['maximum_error']==0 and r['dropped_samples']==r['unchecked_samples']==0 for r in observations)
    results.append(dict(case=case,counts=counts,records=records,stdout=process.stdout))
 print(json.dumps(dict(classification='LOCAL_SUBSTITUTED_PEER_TESTS',windows_workloads=0,results=results)))
if __name__=='__main__':run(*sys.argv[1:])

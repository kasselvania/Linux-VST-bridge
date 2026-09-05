#!/usr/bin/env python3
"""Local platform substitutes through the actual SDK-loaded proxy. Never acceptance."""
import json,mmap,os,pathlib,socket,struct,subprocess,tempfile,threading,time,sys

def exact(s,n):
    b=b''
    while len(b)<n:
        v=s.recv(n-len(b))
        if not v:raise EOFError('peer closed')
        b+=v
    return b

def frame(kind,session,seq,payload=b''):
    return struct.pack('<IHHHHI16sQQQ',0x3141504c,1,2,kind,0,len(payload),session,1,seq,0)+payload

def receive(s):
    h=exact(s,56);magic,major,minor,kind,flags,n,session,instance,seq,parent=struct.unpack('<IHHHHI16sQQQ',h)
    assert (magic,major,minor,flags,instance,parent)==(0x3141504c,1,2,0,1,0) and n<=4040
    return kind,session,seq,exact(s,n)

def peer(directory,scenario,counts):
    until=time.monotonic()+10
    while not (directory/'ap1.control').exists():
        if time.monotonic()>until:raise TimeoutError('endpoint not created')
        time.sleep(.01)
    raw=(directory/'ap1.control').read_bytes();port=struct.unpack_from('<H',raw)[0];session=raw[4:20]
    with (directory/'ap1.audio').open('r+b') as f,mmap.mmap(f.fileno(),4192) as mapping,socket.create_connection(('127.0.0.1',port),5) as s:
        s.settimeout(12);witness=struct.unpack_from('<Q',mapping,32)[0]^0x8d396b274e105ac3
        struct.pack_into('<Q',mapping,40,witness)
        s.sendall(frame(1,session,0,raw[20:]+struct.pack('<II',256,4192)))
        assert receive(s)==(1,session,0,b'')
        assert struct.unpack_from('<Q',mapping,56)[0]==witness^1
        k,token,seq,p=receive(s);assert (k,token,seq,p)==(8,session,1,struct.pack('<I',256));s.sendall(frame(9,session,1));counts['activate']+=1
        assert receive(s)==(10,session,1,b'');s.sendall(frame(11,session,1));counts['start']+=1
        if scenario=='zero':
            assert receive(s)==(12,session,1,b'');s.sendall(frame(13,session,1));assert receive(s)==(14,session,1,b'');s.sendall(frame(15,session,1));assert receive(s)==(5,session,1,b'');s.sendall(frame(6,session,1));return
        if scenario=='positive':
            for seq in range(1,14):
                k,token,number,p=receive(s);assert k==3 and token==session and number==seq
                n=struct.unpack_from('<I',p)[0];gain=struct.unpack_from('<d',p,16)[0];flags=struct.unpack_from('<I',p,24)[0];counts['process']+=1
                for ch in range(2):
                    inp=struct.unpack_from('<'+str(n)+'f',mapping,64+ch*1032+4)
                    struct.pack_into('<'+str(n)+'f',mapping,2128+ch*1032+4,*(x*gain for x in inp))
                s.sendall(frame(4,session,seq,struct.pack('<IIQ',n,2128,3 if gain==0 or flags==3 else 0)))
            assert receive(s)==(12,session,14,b'');s.sendall(frame(13,session,14));assert receive(s)==(14,session,14,b'');s.sendall(frame(15,session,14));assert receive(s)==(5,session,14,b'');s.sendall(frame(6,session,14));return
        k,token,seq,p=receive(s);assert k==3 and token==session and seq==1;counts['process']+=1
        if scenario=='disconnect':return
        if scenario=='timeout':time.sleep(6)
        elif scenario=='invalid':s.sendall(frame(4,session,seq+1,struct.pack('<IIQ',16,2128,0)))
        else:
            for ch in range(2):struct.pack_into('<16f',mapping,2128+ch*1032+4,*([float('nan') if scenario=='nonfinite' else .25]*16))
            s.sendall(frame(4,session,seq,struct.pack('<IIQ',16,2128,1<<40 if scenario=='flags' else 3)))
        # Rejected output must not cause another request, even when the host retries.
        try:assert s.recv(1)==b''
        except ConnectionResetError:pass

def run(host,bundle):
    results=[]
    for case in ['discovery','unavailable','zero','positive','disconnect','timeout','invalid','flags','silence','nonfinite']:
        with tempfile.TemporaryDirectory(prefix='ap2-local-') as temp:
            directory=pathlib.Path(temp);counts={'activate':0,'start':0,'process':0};errors=[]
            env={**os.environ};env.pop('LVB_AP2_SESSION_DIR',None);env.pop('LVB_AP2_SESSION',None)
            thread=None
            if case not in {'discovery','unavailable'}:
                env.update(LVB_AP2_SESSION_DIR=str(directory),LVB_AP2_SESSION=os.urandom(16).hex())
                def target():
                    try:peer(directory,case,counts)
                    except BaseException as error:errors.append(type(error).__name__+': '+str(error))
                thread=threading.Thread(target=target);thread.start()
            p=subprocess.run([host,bundle,case],env=env,capture_output=True,text=True,timeout=25)
            if thread:thread.join(15);assert not thread.is_alive()
            assert p.returncode==0,(case,p.returncode,p.stdout,p.stderr)
            assert not errors,(case,errors)
            assert counts['process']==(13 if case=='positive' else 1 if case in {'disconnect','timeout','invalid','flags','silence','nonfinite'} else 0)
            result={'case':case,'returncode':p.returncode,'peer_counts':counts,'host_records':[json.loads(line) for line in p.stdout.splitlines()]};results.append(result)
    print(json.dumps({'classification':'LOCAL_SUBSTITUTED_PEER_TESTS','windows_workloads':0,'results':results}))
if __name__=='__main__':run(*sys.argv[1:])

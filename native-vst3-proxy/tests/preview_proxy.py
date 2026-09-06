#!/usr/bin/env python3
"""Loaded AP4 private startup and failed-restore checks with a substituted peer."""
import json
import os
import pathlib
import socket
import struct
import subprocess
import sys
import tempfile
import threading
from state_proxy import peer, envelope

def run(host, bundle, audit):
    results=[]
    for case in ('state-gain', 'state-lost-set'):
        with tempfile.TemporaryDirectory(prefix='ap4-preview-') as temp:
            home=pathlib.Path(temp)
            root=home/'AP4-State-Test/preview';root.mkdir(parents=True,mode=0o700)
            session=home/'session';session.mkdir(mode=0o700)
            store=home/'state';store.mkdir();(store/'gain.state').write_bytes(envelope())
            address=root/'owner.sock'
            errors=[];counts=dict(get=0,set=0,activate=0,start=0,stop=0,process=0,zero_frame=0,closed=0)
            with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as listener:
                listener.bind(str(address));address.chmod(0o600);listener.listen(1);listener.settimeout(15)
                def owner():
                    try:
                        with listener.accept()[0] as lease:
                            lease.settimeout(30)
                            assert lease.recv(4)==b'AP4\n'
                            response=(os.urandom(16).hex()+'\n'+str(session)).encode()
                            lease.sendall(struct.pack('<H',len(response))+response)
                            peer(session,case,counts)
                            assert lease.recv(1)==b''
                    except BaseException as error:errors.append(repr(error))
                thread=threading.Thread(target=owner);thread.start()
                env={k:v for k,v in os.environ.items() if not k.startswith('LVB_')}
                env.update(HOME=str(home),LD_PRELOAD=audit,LVB_AP4_STATE_STORE=str(store))
                process=subprocess.run([host,bundle,case],env=env,capture_output=True,text=True,timeout=40)
                thread.join(20)
                assert not thread.is_alive() and not errors,(case,errors,process.stderr)
                assert process.returncode==(0 if case=='state-gain' else 1),(case,process.stdout,process.stderr)
                records=[json.loads(line) for line in (session/'ap3-gui-report.jsonl').read_text().splitlines()]
                if case=='state-gain':
                    comparison=next(r for r in records if r['event']=='ap4_sample_comparison')
                    assert comparison['before_edit_samples']>0 and comparison['maximum_error']==0
                    assert comparison['restored_gain']==.25
                    assert any(r['event']=='ap4_native_state' and r['operation']=='set' and r['gain']==.25 for r in records)
                    assert counts['closed']==1
                else:
                    assert any(r['event']=='ap4_native_error' and r['operation']=='set' for r in records)
                    assert counts['process']==0 and counts['set']==1 and counts['closed']==0
                results.append(dict(case=case,counts=counts,records=records))
    print(json.dumps(dict(classification='LOCAL_SUBSTITUTED_PEER_TESTS',windows_workloads=0,results=results)))

if __name__=='__main__':run(*sys.argv[1:])

#!/usr/bin/env python3
"""Two SDK-loaded instances, one process, independent substituted Windows peers."""
import json
import os
import pathlib
import socket
import struct
import subprocess
import sys
import tempfile
import threading
import time
from state_proxy import peer, envelope


def run(host, bundle, audit):
    results = []
    for case in ('instances-isolation', 'instances-failure'):
        with tempfile.TemporaryDirectory(prefix='ap5-instances-') as temp:
            home = pathlib.Path(temp)
            root = home/'AP4-State-Test/preview'; root.mkdir(parents=True, mode=0o700)
            store = home/'state'; store.mkdir()
            (store/'instance-a.state').write_bytes(envelope(.25))
            (store/'instance-b.state').write_bytes(envelope(.75))
            directories = [home/'a', home/'b']
            for directory in directories: directory.mkdir(mode=0o700)
            errors = []; workers = []
            counts = [dict(get=0, set=0, activate=0, start=0, stop=0, process=0, zero_frame=0, closed=0) for _ in directories]
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
                address = root/'owner.sock'; listener.bind(str(address)); address.chmod(0o600)
                listener.listen(4); listener.settimeout(20)
                def serve(lease, index):
                    try:
                        with lease:
                            lease.settimeout(25)
                            assert lease.recv(4) == b'AP4\n'
                            response = (os.urandom(16).hex()+'\n'+str(directories[index])).encode()
                            lease.sendall(struct.pack('<H', len(response))+response)
                            if case == 'instances-isolation' and index == 1: time.sleep(.5)
                            peer(directories[index], case, counts[index],
                                 fail_after=64 if case == 'instances-failure' and index == 0 else None,
                                 close_delay=.5 if index == 0 else 0)
                            assert lease.recv(1) == b''
                    except BaseException as error: errors.append((index, repr(error)))
                def accept():
                    try:
                        for index in range(2):
                            worker = threading.Thread(target=serve, args=(listener.accept()[0], index))
                            workers.append(worker); worker.start()
                    except BaseException as error: errors.append(('accept', repr(error)))
                admission = threading.Thread(target=accept); admission.start()
                env = {k:v for k,v in os.environ.items() if not k.startswith('LVB_')}
                env.update(HOME=str(home), LD_PRELOAD=audit, LVB_AP4_STATE_STORE=str(store))
                p = subprocess.run([host, bundle, case], env=env, capture_output=True, text=True, timeout=45)
                admission.join(21)
                for worker in workers: worker.join(25)
                assert not admission.is_alive() and all(not w.is_alive() for w in workers)
                assert p.returncode == 0 and not errors, (case, p.returncode, errors, p.stdout, p.stderr)
                reports = [[json.loads(line) for line in (d/'ap3-gui-report.jsonl').read_text().splitlines()] for d in directories]
                summary = next(json.loads(line) for line in p.stdout.splitlines() if '"ap5_two_instances"' in line)
                for index, records in enumerate(reports):
                    assert any(r['event']=='ap4_native_state' and r['operation']=='set' and r['gain']==(.25, .75)[index] for r in records)
                    assert all(r['gain'] != (.75 if index == 0 else .25) for r in records if r['event']=='ap4_native_state')
                survivor = next(r for r in reports[1] if r['event']=='ap4_sample_comparison')
                assert survivor['maximum_error']==0 and survivor['edits']==0 and survivor['before_edit_samples']>0
                assert counts[1]['closed']==1
                if case=='instances-failure':
                    assert counts[1]['process']>counts[0]['process']
                    assert summary['a_failed'] and counts[0]['closed']==0
                    assert any(r['event']=='ap4_native_error' for r in reports[0])
                else:
                    assert counts[0]['closed']==1 and not summary['a_failed']
                results.append(dict(case=case,counts=counts,summary=summary,per_instance_reports=reports))
    print(json.dumps(dict(classification='LOCAL_SUBSTITUTED_PEER_TESTS',windows_workloads=0,results=results)))


if __name__ == '__main__': run(*sys.argv[1:])

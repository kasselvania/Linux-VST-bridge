#!/usr/bin/env python3
"""Focused SDK recovery checks; substituted peers, no Windows workloads."""
import json, os, pathlib, shutil, socket, struct, subprocess, sys, tempfile, threading
from state_proxy import peer, envelope

def run(host, bundle, audit, cases=None):
    results = []
    for case in cases or ('recovery-complete', 'recovery-missing', 'recovery-bad-readback', 'recovery-uncertain'):
        with tempfile.TemporaryDirectory(prefix='ap6-recovery-') as temp:
            home = pathlib.Path(temp)
            root = home/'AP4-State-Test/preview'; root.mkdir(parents=True, mode=0o700)
            output = root/'results'; output.mkdir(mode=0o700)
            store = home/'state'; store.mkdir()
            (store/'instance-a.state').write_bytes(envelope(.25, .125))
            (store/'instance-b.state').write_bytes(envelope(.75))
            expected = 2 if case in ('recovery-missing', 'recovery-uncertain') else 3
            errors = []; workers = []; counts = []; reports = []; admissions = []
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
                address = root/'owner.sock'; listener.bind(str(address)); address.chmod(0o600)
                listener.listen(4); listener.settimeout(20)
                def serve(lease, index):
                    try:
                        directory = home/str(index); directory.mkdir(mode=0o700)
                        counter = dict(get=0,set=0,activate=0,start=0,stop=0,process=0,zero_frame=0,closed=0)
                        counts.append(counter)
                        token = os.urandom(16).hex(); reports.append(output/('native-'+token+'.jsonl'))
                        with lease:
                            lease.settimeout(25)
                            assert lease.recv(4) == b'AP4\n'
                            response = (token+'\n'+str(directory)).encode()
                            lease.sendall(struct.pack('<H',len(response))+response)
                            mode = ('recovery-bad-readback' if index == 2 and case == 'recovery-bad-readback' else 'recovery-peer')
                            try:
                                peer(directory, mode, counter, fail_after=64 if index == 0 else None)
                            except (BrokenPipeError, ConnectionResetError):
                                if mode != 'state-callback-fault': raise
                            assert lease.recv(1) == b''
                            if index == 0 and case == 'recovery-uncertain':
                                lease.sendall(b'F')
                            else:
                                shutil.rmtree(directory)
                                admissions.append(index)
                                lease.sendall(b'R')
                    except BaseException as error: errors.append((index,repr(error)))
                def accept():
                    try:
                        for index in range(expected):
                            lease,_ = listener.accept()
                            if index == 2: assert 0 in admissions, 'replacement before retired endpoint'
                            worker = threading.Thread(target=serve,args=(lease,index)); workers.append(worker); worker.start()
                    except BaseException as error: errors.append(('accept',repr(error)))
                t = threading.Thread(target=accept); t.start()
                env = {k:v for k,v in os.environ.items() if not k.startswith('LVB_')}
                env.update(HOME=str(home),LD_PRELOAD=audit,LVB_AP4_STATE_STORE=str(store))
                process = subprocess.run([host,bundle,case],env=env,capture_output=True,text=True,timeout=60)
                t.join(21)
                for worker in workers: worker.join(25)
                assert not t.is_alive() and all(not w.is_alive() for w in workers)
                assert process.returncode == 0 and not errors,(case,errors,process.stdout,process.stderr)
                records = [[json.loads(line) for line in path.read_text().splitlines()] for path in reports if path.exists()]
                fault = next(r for group in records for r in group if r['event'] == 'ap5_worker_fault')
                assert fault['detail'] and fault['fault'] != 0
                progress = next(r for group in records for r in group if r['event'] == 'ap7_fault_progress')
                assert progress['context_ready'] and progress['epoch'] == 1
                lifecycle = [r for group in records for r in group if r['event']=='ap3_proxy_lifecycle']
                assert len(lifecycle) == 2
                failed = next(r for r in lifecycle if r['callback_rejections'])
                healthy = next(r for r in lifecycle if not r['callback_rejections'])
                assert failed['discontinuities'] == 1 and failed['rejected_silent_frames'] > 0
                assert healthy['discontinuities'] == healthy['rejected_silent_frames'] == 0
                assert healthy['priming_frames'] == 1024
                if case == 'recovery-complete':
                    assert failed['priming_frames'] == 2048
                summary = next(json.loads(line) for line in process.stdout.splitlines() if '"ap6_recovery"' in line)
                if case == 'recovery-complete':
                    assert summary['recovered_samples'] >= 8192 and counts[2]['set'] == 1 and counts[2]['closed'] == 1
                else: assert summary['recovered_samples'] == 0
                assert summary['sibling_samples'] > 0 and counts[1]['closed'] == 1
                results.append(dict(case=case,summary=summary,counts=counts,first_fault=fault,records=records))
    print(json.dumps(dict(classification='LOCAL_SUBSTITUTED_PEER_TESTS',windows_workloads=0,results=results)))

if __name__ == '__main__': run(*sys.argv[1:])

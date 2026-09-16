#!/usr/bin/env python3
"""Account-free multistage payload through the actual installer owner on Linux.
Run on the declared Deck with staged source, never install it as manager software.
Only scratch prefixes and exact generated cgroups are created. No GUI input.
"""
import argparse,hashlib,json,os,pathlib,shutil,subprocess,time
CASES=('success','payload_failure','service_failure','postlaunch_failure','outer_first','failure_while_alive','short_lived','handoff','same_names','cancel_after_failure','noise','nothing_installed')
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def run(runtime,payload,output,case):
    os.umask(0o077);output.mkdir(mode=0o700,parents=True,exist_ok=False)
    home=pathlib.Path.home();managed=home/'.local/share/linux-vst-bridge/managed';cli=home/'.local/bin/linux-vst-bridge'
    cap=json.loads(subprocess.check_output([str(cli),'capacity']))['capacity']
    assert cap['dsp']==0 and cap['maintenance']==0 and not cap['cleanup_unconfirmed']
    registry=json.loads((managed/'registry.json').read_text())
    runner=next(iter(registry['classes'].values()))['registration']['environment']['runner']
    root=output/'environment';root.mkdir(mode=0o700)
    for name in ('home','compatdata','runtime-var','host-cache','host-config','host-data','host-tmp','client'):(root/name).mkdir(mode=0o700)
    (root/'operation.lock').touch(mode=0o600)
    exe=output/'fixture.exe';shutil.copyfile(payload,exe);exe.chmod(0o400);exe.with_suffix('.exe.case').write_text(case+'\n')
    op=os.urandom(16).hex();report=output/'result.json';spec=output/'spec.json'
    spec.write_text(json.dumps({'schema':2,'operation':op,'environment':{'id':op,'root':str(root),'revision':1,'runner':runner},'installer':{'path':str(exe),'sha256':digest(exe)},'format':'pe_executable','report':str(report)}))
    unit='linux-vst-bridge-installer-'+op+'.service';began=time.monotonic();cancel=False;early_failure=None
    subprocess.run(['systemd-run','--user','--collect','--property=UMask=0077','--property=KillMode=control-group','--property=TimeoutStopSec=20','--property=StandardOutput=null','--property=StandardError=null','--unit='+unit,'/usr/bin/python3',str(runtime/'session.py'),'--install',str(spec)],check=True)
    try:
        while time.monotonic()-began<110:
            if report.exists():
                try:value=json.loads(report.read_text())
                except json.JSONDecodeError:continue
                fault=value.get('transaction',{}).get('first_failure')
                if fault and fault['relationship']=='descendant' and early_failure is None:early_failure=fault
                if case=='cancel_after_failure' and early_failure and not cancel:
                    cancel=True;subprocess.run(['systemctl','--user','stop',unit],check=True,timeout=30)
                if value.get('cleanup_confirmed'):break
            time.sleep(.05)
    finally:
        if subprocess.run(['systemctl','--user','is-active','--quiet',unit]).returncode==0:
            subprocess.run(['systemctl','--user','stop',unit],check=True,timeout=30)
    final=json.loads(report.read_text());private=json.loads((output/(op+'-transaction-private.json')).read_text())
    log=(output/(op+'-private.log')).read_bytes()
    rows=private['ledger']['processes'];statuses=[r['linux_exit']['status'] for r in rows if r['linux_exit']]
    survivors=[]
    for r in rows:
        try:
            st=(pathlib.Path('/proc')/str(r['pid'])/'stat').read_text()
            if int(st.rsplit(')',1)[1].split()[19])==r['start_ticks']:survivors.append(r)
        except FileNotFoundError:pass
    proof={'schema':1,'case':case,'production_supervisor':True,'source':{n:digest(runtime/n) for n in ('session.py','ownership.py')},'payload_sha256':digest(exe),
        'duration_seconds':time.monotonic()-began,'ready':b'IS1_FIXTURE_V1 kind=ready' in log,
        'fixture_events':[line.decode(errors='replace') for line in log.splitlines() if line.startswith(b'IS1_FIXTURE_V1')],
        'result':final,'status_census':statuses,'early_failure':early_failure,'cancel_requested':cancel,'same_identity_survivors':len(survivors),
        'unit_inactive':subprocess.run(['systemctl','--user','is-active','--quiet',unit]).returncode!=0}
    (output/'proof-private.json').write_text(json.dumps(proof,indent=2))
    assert final['cleanup_confirmed'] and final['owned_live']==0 and not survivors
    print(json.dumps({'case':case,'ready':proof['ready'],'state':final['state'],'outer':final['raw_exit'],'transaction':final.get('transaction'),'statuses':statuses,'cancel':cancel}))
if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--runtime',type=pathlib.Path,required=True);parser.add_argument('--payload',type=pathlib.Path,required=True);parser.add_argument('--output',type=pathlib.Path,required=True);parser.add_argument('--case',choices=CASES,required=True);a=parser.parse_args()
    run(a.runtime.resolve(),a.payload.resolve(),a.output.resolve(),a.case)

#!/usr/bin/env python3
"""Account-free multistage payload through the actual installer owner on Linux.
Run on the declared Deck with staged source, never install it as manager software.
Only scratch prefixes and exact generated cgroups are created. No GUI input.
"""
import argparse,hashlib,json,os,pathlib,re,shutil,subprocess,time
CASES=('success','payload_failure','service_failure','postlaunch_failure','outer_first','failure_while_alive','short_lived','handoff','same_names','cancel_after_failure','noise','nothing_installed')
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def run(runtime,payload,output,case,legacy=False,direct_msi=False):
    os.umask(0o077)
    assert shutil.disk_usage(output.parent).free>2*1024**3,'scratch free-space preflight'
    output.mkdir(mode=0o700,parents=True,exist_ok=False)
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
    spec.write_text(json.dumps({'schema':2,'operation':op,'environment':{'id':op,'root':str(root),'revision':1,'runner':runner},'installer':{'path':str(exe),'sha256':digest(exe)},'format':'msi_compound' if direct_msi else 'pe_executable','report':str(report)}))
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
            stopped=subprocess.run(['systemctl','--user','stop',unit],timeout=30)
            if stopped.returncode:
                # --collect can remove the unit between is-active and Stop.
                done=json.loads(report.read_text())
                assert done.get('cleanup_confirmed') and done.get('owned_live')==0 and subprocess.run(['systemctl','--user','is-active','--quiet',unit]).returncode!=0
    final=json.loads(report.read_text());private=json.loads((output/(op+'-transaction-private.json')).read_text()) if not legacy else {'ledger':{'processes':[]},'windows_trace':{'processes':[]}}
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
    if direct_msi:
        assert final['raw_exit']==0 and final['transaction']['diagnostics']['msi']['retained_bytes']>0
    elif not legacy:verify(case,proof,private)
    # Only this generated scratch prefix, after exact positive retirement.
    assert root.resolve()==root and json.loads(spec.read_text())['environment']['root']==str(root)
    shutil.rmtree(root);proof['scratch_prefix_removed']=True
    (output/'proof-private.json').write_text(json.dumps(proof,indent=2))
    print(json.dumps({'case':case,'ready':proof['ready'],'state':final['state'],'outer':final['raw_exit'],'transaction':final.get('transaction'),'statuses':statuses,'cancel':cancel}))
def verify(case,proof,private):
    result=proof['result'];tx=result['transaction'];failure=tx['first_failure']
    assert proof['ready'] and tx['persistence_failures']==0 and tx['dropped_process_observations']==0
    expected={'payload_failure':37,'service_failure':73,'postlaunch_failure':41,'failure_while_alive':39,'short_lived':43,'same_names':47,'cancel_after_failure':39}
    if case in expected:
        assert failure and failure['status']==expected[case] and failure['domain']=='wine_self_exit_observation',(case,failure)
    else:assert failure is None,(case,failure)
    assert tx['durable_installation']==('installed' if case in ('success','service_failure','postlaunch_failure','noise') else 'not_installed')
    if case=='cancel_after_failure':
        assert proof['cancel_requested'] and result['state']=='cancelled' and proof['early_failure']==failure
    elif case in expected:assert result['raw_exit']==2
    else:assert result['raw_exit']==0
    trace=[r for r in private['windows_trace']['processes'] if r['target_tree']]
    assert len({(r['epoch'],r['windows_pid'],r['creation_ordinal']) for r in trace})==len(trace)
    assert all(r['linux_identity']=='unavailable_no_cross_id_inference' for r in trace)
    # Independent source-owned oracle. These are Windows IDs on both sides,
    # never a numeric Windows/Linux identity join or production role authority.
    events=[]
    for line in proof['fixture_events']:
        match=re.fullmatch(r'IS1_FIXTURE_V1 kind=(\w+) pid=(\d+) role=(\w+) code=(\d+) tick=(\d+)',line)
        assert match,line
        kind,pid,role,code,tick=match.groups()
        events.append(dict(kind=kind,pid=int(pid),role=role,code=int(code),tick=int(tick)))
    prerequisite=[e for e in events if e['role']=='prerequisite' and e['kind']=='exit']
    assert len(prerequisite)==1 and prerequisite[0]['code']==0
    for event in [e for e in events if e['kind']=='exit']:
        observed=[r for r in trace if r['windows_pid']==event['pid'] and r['self_exit']]
        assert len(observed)==1 and observed[0]['self_exit']['status']==event['code'],(case,event)
    if case=='same_names':
        children=[r for r in trace if any(e['kind']=='exit' and e['role']=='payload' and e['pid']==r['windows_pid'] for e in events)]
        assert len(children)==2 and children[0]['windows_pid']!=children[1]['windows_pid']
        assert children[0]['image_request']==children[1]['image_request']
    if case=='postlaunch_failure':
        post=[r for r in trace if any(e['kind']=='exit' and e['role']=='postinstall_launch' and e['pid']==r['windows_pid'] for e in events)]
        assert len(post)==1 and post[0]['image_identity']['authority']=='same_open_file_at_launch_request_not_mapped'
        assert any(post[0]['image_identity']['sha256']==private['after']['files'][p]['sha256'] for a in private['durable_installation']['application_registrations'] for p in a['images'])
    if case=='handoff':
        relaunch=[r for r in trace if any(e['kind']=='exit' and e['role']=='relaunch' and e['pid']==r['windows_pid'] for e in events)]
        assert len(relaunch)==1
        assert any(r['creation_ordinal']==relaunch[0]['parent_ordinal'] and not r['target_root'] for r in trace)
    if case=='failure_while_alive':
        root=next(r for r in trace if r['target_root']);failed=next(r for r in trace if r['self_exit'] and r['self_exit']['status']==39)
        assert float(root['self_exit']['timestamp'])-float(failed['self_exit']['timestamp'])>=1
    if case=='cancel_after_failure':
        assert any(e['kind']=='holding' for e in events) and private['ledger']['first_failure'] is None
    if case in ('outer_first','handoff'):
        roots=[r for r in trace if r['target_root']];children=[r for r in trace if not r['target_root'] and r['self_exit']]
        assert roots and children and max(float(r['self_exit']['timestamp']) for r in children)>float(roots[-1]['self_exit']['timestamp'])
    if case=='noise':assert tx['diagnostics']['accessibility_observer']['retained_bytes']>0

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--runtime',type=pathlib.Path,required=True);parser.add_argument('--payload',type=pathlib.Path,required=True);parser.add_argument('--output',type=pathlib.Path,required=True);parser.add_argument('--case',choices=CASES,required=True);parser.add_argument('--legacy-baseline',action='store_true');parser.add_argument('--direct-msi',action='store_true');a=parser.parse_args()
    run(a.runtime.resolve(),a.payload.resolve(),a.output.resolve(),a.case,a.legacy_baseline,a.direct_msi)

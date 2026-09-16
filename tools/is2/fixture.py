#!/usr/bin/env python3
"""Source-owned IS2 diagnostics. No commercial import, installed software or GUI input.
Each run uses production --install, one dedicated cgroup and disposable prefix.
"""
import argparse,hashlib,json,os,pathlib,re,shutil,subprocess,time
CASES=('no_app','cooperative','ignores','exit_between','stale_pid','same_name','self','stale_mutex','stale_window','helper_zero','stale_recheck','access_denied','cancel_after','cleanup_preserves','powershell','wrapper1','wrapper3','exit23','contract')
def digest(p):
    with p.open('rb') as source:return hashlib.file_digest(source,'sha256').hexdigest()
def check_oracle(case, lines):
    rows=[]
    for line in lines:
        if line.startswith('IS2_CONTRACT'):continue
        match=re.fullmatch(r'IS2_API_V1 case=([a-z0-9_]+) stage=([a-z_]+) pid=([0-9]+) generation=([0-9]+) matches=(-?[0-9]+) outcome=([a-z0-9_]+) status=([0-9]+) tick=([0-9]+)',line)
        assert match and match[1]==case, 'malformed or cross-case oracle'
        rows.append(dict(zip(('case','stage','pid','generation','matches','outcome','status','tick'),match.groups())))
    def one(stage):
        found=[r for r in rows if r['stage']==stage];assert len(found)==1,(stage,found);return found[0]
    assert rows and rows[0]['stage']=='ready'
    if case in ('contract','exit23','wrapper1','wrapper3'):return {'schema':1,'case':case,'authority':'source_owned_fixture_only','result':'observed'}
    if case=='powershell':
        scripts=[r for r in rows if r['stage']=='script'];assert len(scripts)==4
        assert [r['outcome'] for r in scripts]==['probe0','probe1','probe2','probe3']
        sentinel=one('sentinel')
        return {'schema':1,'case':case,'authority':'source_owned_fixture_only','script_exit_statuses':[int(r['status']) for r in scripts], 'sentinel_created':sentinel['matches']=='1','requested_first_exit':37}
    present=one('presence');close=one('close')
    if case in ('no_app','stale_mutex'):
        assert present['matches']=='0' and close['outcome']=='not_requested' and one('recheck')['matches']=='0'
    elif case=='self':assert present['matches']=='1' and close['outcome']=='refused_identity'
    else:
        assert present['matches']=='1' and int(present['pid'])>0 and int(present['generation'])>0
        assert close['pid']==present['pid']
        recheck=one('recheck');assert (recheck['pid'],recheck['generation'])==(present['pid'],present['generation'])
        absent=case in ('cooperative','same_name','exit_between','stale_window','stale_recheck')
        assert recheck['matches']==('0' if absent else '1'),recheck
        expected=('refused_identity' if case in ('exit_between','stale_pid','stale_window') else 'access_denied' if case=='access_denied' else 'helper_zero_no_close' if case=='helper_zero' else 'request_accepted')
        assert close['outcome']==expected,close
        if case=='access_denied':assert int(close['status'])==5
        if case=='same_name':
            other=one('unrelated');assert other['matches']=='1' and other['pid']!=present['pid']
        if case=='stale_recheck':assert one('cached_recheck')['outcome']=='rejected_stale_snapshot'
        if case=='cancel_after':assert one('holding')['matches']=='1'
        else:assert one('cleanup')['matches']=='0'
    # Sanitized exact source-owned result. Never input to vendor classification.
    return {'schema':1,'case':case,'authority':'source_owned_fixture_only','presence_count':int(present['matches']),'close':close['outcome'],'close_status_domain':'win32_api' if case=='access_denied' else 'fixture_observation','close_status':int(close['status']),'recheck':[r['outcome'] for r in rows if r['stage']=='recheck'],'first_close_failure':case in ('ignores','stale_pid','self','stale_window','helper_zero','access_denied','cancel_after','cleanup_preserves'),'cause':'unestablished'}
def run(runtime,payload,adapter,output,case):
    os.umask(0o077);assert case in CASES;assert shutil.disk_usage(output.parent).free>2*1024**3
    output.mkdir(mode=0o700,parents=True,exist_ok=False)
    home=pathlib.Path.home();managed=home/'.local/share/linux-vst-bridge/managed';cli=home/'.local/bin/linux-vst-bridge'
    cap=json.loads(subprocess.check_output([str(cli),'capacity']))['capacity'];assert cap['dsp']==0 and cap['maintenance']==0 and not cap['cleanup_unconfirmed']
    runner=next(iter(json.loads((managed/'registry.json').read_text())['classes'].values()))['registration']['environment']['runner']
    root=output/'environment';root.mkdir(mode=0o700)
    for name in ('home','compatdata','runtime-var','host-cache','host-config','host-data','host-tmp','client'):(root/name).mkdir(mode=0o700)
    (root/'operation.lock').touch(mode=0o600);exe=output/'fixture.exe';shutil.copyfile(payload,exe);exe.chmod(0o400);exe.with_suffix('.exe.case').write_text(case+'\n')
    op=os.urandom(16).hex();report=output/'result.json';spec=output/'spec.json'
    value={'schema':2,'operation':op,'environment':{'id':op,'root':str(root),'revision':1,'runner':runner},'installer':{'path':str(exe),'sha256':digest(exe)},'format':'pe_executable','report':str(report)}
    if adapter:value['installer_launch']={'path':str(adapter),'sha256':digest(adapter)}
    spec.write_text(json.dumps(value));unit='linux-vst-bridge-installer-'+op+'.service';began=time.monotonic();cancel=False
    subprocess.run(['systemd-run','--user','--collect','--property=UMask=0077','--property=KillMode=control-group','--property=TimeoutStopSec=20','--property=StandardOutput=null','--property=StandardError=null','--unit='+unit,'/usr/bin/python3',str(runtime/'session.py'),'--install',str(spec)],check=True)
    try:
        while time.monotonic()-began<110:
            if report.exists():
                value=json.loads(report.read_text())
                if case=='cancel_after' and value.get('startup',{}).get('target_observation')=='observed' and time.monotonic()-began>18 and not cancel:
                    cancel=True;subprocess.run(['systemctl','--user','stop',unit],check=True,timeout=30)
                if value.get('cleanup_confirmed'):break
            time.sleep(.05)
    finally:
        if subprocess.run(['systemctl','--user','is-active','--quiet',unit]).returncode==0:
            stopped=subprocess.run(['systemctl','--user','stop',unit],timeout=30)
            if stopped.returncode:
                done=json.loads(report.read_text());assert done.get('cleanup_confirmed') and done.get('owned_live')==0 and subprocess.run(['systemctl','--user','is-active','--quiet',unit]).returncode!=0
    return finish(runtime,adapter,output,case,began,cancel)
def finish(runtime,adapter,output,case,began,cancel):
    root=output/'environment';exe=output/'fixture.exe';report=output/'result.json';op=json.loads((output/'spec.json').read_text())['operation']
    final=json.loads(report.read_text());private=json.loads((output/(op+'-transaction-private.json')).read_text());log=(output/(op+'-private.log')).read_bytes()
    events=[line.decode() for line in log.splitlines() if line.startswith((b'IS2_API_V1',b'IS2_CONTRACT'))]
    assert final['cleanup_confirmed'] and final['owned_live']==0
    survivors=[]
    for r in private['ledger']['processes']:
        try:
            if int((pathlib.Path('/proc')/str(r['pid'])/'stat').read_text().rsplit(')',1)[1].split()[19])==r['start_ticks']:survivors.append(r)
        except FileNotFoundError:pass
    assert not survivors
    contract=exe.with_suffix('.exe.contract')
    contract_hash=digest(contract) if contract.exists() else None
    runtime_images={}
    for arch in ('system32','syswow64'):
        image=root/'compatdata/pfx/drive_c/windows'/arch/'WindowsPowerShell/v1.0/powershell.exe'
        if image.exists():runtime_images[arch]={'sha256':digest(image),'size':image.stat().st_size}
    proof={'case':case,'contract_sha256':contract_hash,'powershell_images':runtime_images,'source':{n:digest(runtime/n) for n in ('session.py','ownership.py')},'adapter_sha256':digest(adapter) if adapter else None,'payload_sha256':digest(exe),'result':final,'fixture_api_oracle':events,'oracle_is_not_production_authority':True,'cancel_requested':cancel,'duration_seconds':time.monotonic()-began if began is not None else None,'same_identity_survivors':0}
    (output/'proof-private.json').write_text(json.dumps(proof,indent=2))
    # Cleanup is owned even when an attribution assertion fails; retain proof.
    assert root.resolve()==root;shutil.rmtree(root);proof['scratch_prefix_removed']=True;(output/'proof-private.json').write_text(json.dumps(proof,indent=2))
    if adapter:assert final['transaction']['launch_binding']['status']=='bound',final['transaction']['launch_binding']
    if case=='exit23':assert final['raw_exit']==23
    elif case=='cancel_after':assert final['state']=='cancelled'
    else:assert final['raw_exit']==0
    assert events,'source-owned payload did not report'
    proof['verified_oracle']=check_oracle(case,events)
    (output/'proof-private.json').write_text(json.dumps(proof,indent=2))
    print(json.dumps({'case':case,'outer':final['raw_exit'],'binding':final['transaction'].get('launch_binding'),'windows_roots':sum(r['target_root'] for r in private['windows_trace']['processes']),'presence_classes':final['transaction'].get('presence_close'),'cleanup':final['cleanup_confirmed'],'oracle':proof['verified_oracle']}))
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--runtime',type=pathlib.Path,required=True);p.add_argument('--payload',type=pathlib.Path,required=True);p.add_argument('--adapter',type=pathlib.Path);p.add_argument('--output',type=pathlib.Path,required=True);p.add_argument('--case',choices=CASES,required=True);a=p.parse_args();run(a.runtime.resolve(),a.payload.resolve(),a.adapter.resolve() if a.adapter else None,a.output.resolve(),a.case)

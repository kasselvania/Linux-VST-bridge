"""Bounded inherited/software/restored generated proof; never a vendor campaign."""
import os,pathlib,shutil,subprocess,sys,time
from package import verify,read,digest,publish,canonical
from readback import readback

def compare(rows,identity):
    if len(rows)!=3 or [r['mode'] for r in rows]!=['inherited','software_rendering','inherited'] or len({r['operation'] for r in rows})!=3:raise ValueError('campaign_order')
    for r in rows:
        if r['identity']!=identity or not r['cleanup_confirmed'] or r['owned_live']!=0 or r['outer_exit']!=0 or not r['prefix_removed'] or not r['renderer']['complete']:raise ValueError('campaign_authority')
        if r['effective']['policy']!=r['mode'] or r['renderer']['launch_binding']['status']!='bound':raise ValueError('campaign_policy_root')
    if [r['renderer']['cause'] for r in rows]!=['gpu','unresolved','gpu']:raise ValueError('campaign_behavior')
    return 'NAUI2_GENERATED_PRODUCTION_DIFFERENTIAL_PASSED'

def run(package,out,seal):
    p=pathlib.Path(package).resolve();out=pathlib.Path(out).resolve();m=verify(p,seal);context=read(p/'context.private.json')
    before=readback()
    if canonical(before)!=canonical(context):raise ValueError('fixture_current_state_drift')
    out.mkdir(mode=0o700);publish(out/'before.private.json',before)
    expected='proton-11.0-2c-25118279-slr4-4.0.20260805.254769'
    runners={canonical(v['registration']['environment']['runner']):v['registration']['environment']['runner'] for v in context['registry']['classes'].values() if v['registration']['environment']['runner']['id']==expected}
    if len(runners)!=1:raise ValueError('runner_generation_ambiguous')
    runner=next(iter(runners.values()))
    for artifact in runner['files']:
        if digest(artifact['path'])!=artifact['sha256']:raise ValueError('runner_bytes_changed')
    identity={'seal':seal,'source':m['source'],'files':m['files'],'runner_sha256':__import__('hashlib').sha256(canonical(runner)).hexdigest()}
    rows=[]
    for index,mode in enumerate(('inherited','software_rendering','inherited','inherited')):
        verify(p,seal);op=os.urandom(16).hex();d=out/str(index);d.mkdir(mode=0o700);root=d/'environment';root.mkdir(mode=0o700)
        for n in ('home','compatdata','runtime-var','host-cache','host-config','host-data','host-tmp','client'):(root/n).mkdir(mode=0o700)
        (root/'operation.lock').touch(mode=0o600)
        env={'id':op,'root':str(root),'revision':1,'runner':runner,'family':'naui2_fixture'}
        # Environment's full Rust shape is copied from a registered environment,
        # replacing only this disposable location/identity and revision.
        template=next(v['registration']['environment'] for v in context['registry']['classes'].values() if v['registration']['environment']['runner']==runner)
        env=dict(template,id=op,root=str(root),revision=1)
        publish(root/'environment.json',env)
        payload=d/'fixture.exe';shutil.copyfile(p/'payload.exe',payload);payload.chmod(0o400)
        (d/'case.txt').write_text('hold' if index==3 else 'differential')
        installation=d/'fixture-installation.json';publish(installation,{'source_owned':True})
        app={'schema':1,'id':'native-access','environment':env,'files':{'Native Access.exe':{'artifact':{'path':str(payload),'sha256':m['files']['payload.exe']},'size':m['payload_size']}},
             'installation':{'path':str(installation),'sha256':digest(installation)},'observation_sha256':'0'*64,'source_seal_sha256':seal}
        sw=dict(context['software'])
        for key,name in [('manager','binding-owner'),('supervisor','session.py'),('ownership','ownership.py'),('installer_launch','adapter.exe')]:sw[key]={'path':str(p/name),'sha256':m['files'][name]}
        publish(d/'application.private.json',app);publish(d/'software.private.json',sw)
        report=d/'result.json';spec=d/'spec.private.json'
        subprocess.run([str(p/'binding-owner'),str(d/'application.private.json'),str(d/'software.private.json'),op,mode,str(report),str(spec)],check=True,timeout=20)
        unit='linux-vst-bridge-renderer-'+op+'.service'
        subprocess.run(['systemd-run','--user','--collect','--property=UMask=0077','--property=KillMode=control-group','--property=TimeoutStopSec=30','--property=StandardOutput=null','--property=StandardError=null','--unit='+unit,'/usr/bin/python3','-B',str(p/'supervise.py'),seal,str(spec)],check=True,timeout=15)
        completed=False;stop_requested=False;deadline=time.monotonic()+180
        try:
            while time.monotonic()<deadline:
                if report.exists():
                    observed=read(report)
                    if index==3 and not stop_requested and observed.get('effective') and observed['renderer']['cause']=='gpu':
                        subprocess.run(['systemctl','--user','stop',unit],check=True,timeout=35);stop_requested=True
                    if observed.get('cleanup_confirmed'):completed=True;break
                time.sleep(.1)
        finally:
            active=lambda:subprocess.run(['systemctl','--user','is-active','--quiet',unit]).returncode==0
            if not completed and active():subprocess.run(['systemctl','--user','stop',unit],check=True,timeout=35)
            deadline=time.monotonic()+10
            while active() and time.monotonic()<deadline:time.sleep(.05)
            if active():raise ValueError('unit_retirement_unconfirmed')
        r=read(report)
        if not r['cleanup_confirmed'] or r['owned_live']!=0:raise ValueError('application_cleanup_unconfirmed')
        shutil.rmtree(root)
        row={**r,'mode':mode,'identity':identity,'prefix_removed':True,'private_result_sha256':digest(report)}
        publish(d/'proof.json',row)
        if index==3:
            if not stop_requested or r['state']!='cancelled' or r['renderer']['cause']!='gpu' or not r['effective']:raise ValueError('fixture_stop_did_not_preserve_failure')
            stopped=row
        else:
            rows.append(row)
            if r['state']!='completed':raise ValueError('fixture_session_failed')
    disposition=compare(rows,identity);after=readback()
    if canonical(before)!=canonical(after):raise ValueError('fixture_preservation_changed')
    publish(out/'after.private.json',after)
    result={'schema':1,'disposition':disposition,'identity':identity,'sessions':rows,'cancel_after_failure':stopped,
            'preservation':{'system':after['system'],'capture':after['capture'],'retained_files':len(after['retained']),'projects':len(after['projects']),'unchanged':True},
            'commercial_launch':False,'installed_software_changed':False}
    publish(out/'generated.json',result);return result
if __name__=='__main__':run(*sys.argv[1:])

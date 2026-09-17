"""Generated installation/SCM lifecycle; no Native Access or real daemon input."""
import hashlib,json,os,pathlib,shutil,subprocess,time
from owner_package import verify,digest,read,publish,canonical
from readback import readback

def run(package,seal,out):
 p=pathlib.Path(package).resolve();manifest=verify(p,seal);d=pathlib.Path(out).resolve();os.umask(0o077);d.mkdir(mode=0o700)
 before=readback();publish(d/'before.private.json',before)
 runners={canonical(v['registration']['environment']['runner']):v['registration']['environment'] for v in before['registry']['classes'].values() if v['registration']['environment']['runner']['id']=='proton-11.0-2c-25118279-slr4-4.0.20260805.254769'}
 if len(runners)!=1:raise ValueError('runner_ambiguous')
 template=next(iter(runners.values()));rows=[]
 for scenario in ('absent','unregistered','stopped','not_ready','cancel'):
  verify(p,seal);case=d/scenario;case.mkdir(mode=0o700);root=case/'environment';root.mkdir(mode=0o700)
  for name in ('home','compatdata','runtime-var','host-cache','host-config','host-data','host-tmp','client'):(root/name).mkdir(mode=0o700)
  (root/'operation.lock').touch(mode=0o600);op=os.urandom(16).hex();env=dict(template,id=op,root=str(root),revision=1);publish(root/'environment.json',env)
  publish(case/'case.json',{'case':scenario});appfile=case/'source-owned.exe';shutil.copyfile(p/'Setup.exe',appfile);appfile.chmod(0o400)
  install=case/'fixture-installation.json';publish(install,{'generated':True})
  app={'schema':1,'id':'nad1-source-owned','environment':env,'files':{'Native Access.exe':{'artifact':{'path':str(appfile),'sha256':digest(appfile)},'size':appfile.stat().st_size}},'installation':{'path':str(install),'sha256':digest(install)},'observation_sha256':'0'*64,'source_seal_sha256':seal}
  sw=dict(before['software'])
  for k,n in [('manager','binding-owner'),('supervisor','session.py'),('ownership','ownership.py'),('installer_launch','adapter.exe')]:sw[k]={'path':str(p/n),'sha256':manifest['files'][n]}
  publish(case/'app.private.json',app);publish(case/'software.private.json',sw);manager=case/'manager';report=manager/'vendor-applications/native-access-dependency/operations'/op/'result.json';spec=case/'spec.private.json'
  subprocess.run([str(p/'binding-owner'),str(case/'app.private.json'),str(case/'software.private.json'),op,'software_rendering',str(report),str(spec)],check=True,timeout=20)
  # The shared manager example expects supervise.py; the package exposes only the
  # fixed sealed owner_supervise.py, selected by this source-owned Rust build.
  subprocess.run([str(p/'binding-owner'),'submit',str(manager),str(spec),str(p),seal],check=True,timeout=30)
  unit='linux-vst-bridge-dependency-'+op+'.service';deadline=time.monotonic()+320;stopped=False
  try:
   while time.monotonic()<deadline:
    if report.exists():break
    if scenario=='cancel' and list(report.parent.glob(op+'-dependency-stage-1.json')) and not stopped:
     subprocess.run([str(p/'binding-owner'),'stop',str(manager),op],check=True,timeout=40);stopped=True
    time.sleep(.2)
  finally:
   subprocess.run([str(p/'binding-owner'),'stop',str(manager),op],check=True,timeout=40)
  r=read(report) if report.exists() else read(report.parent/'recovery-result.json')
  if not r.get('cleanup_confirmed') or r.get('owned_live')!=0:raise ValueError('fixture_cleanup')
  state=subprocess.run(['systemctl','--user','show',unit,'--property=LoadState,ActiveState,MainPID,ControlPID,ControlGroup'],capture_output=True,timeout=15).stdout.decode()
  if dict(x.split('=',1) for x in state.splitlines())!={'LoadState':'not-found','ActiveState':'inactive','MainPID':'0','ControlPID':'0','ControlGroup':''}:raise ValueError('unit_not_absent')
  events=root/'compatdata/pfx/drive_c/NAD1Fixture/events.private'
  if events.exists():shutil.copyfile(events,case/'events.private')
  shutil.rmtree(root);row={'case':scenario,'operation':op,'result':r,'result_sha256':digest(report if report.exists() else report.parent/'recovery-result.json'),'prefix_removed':True,'unit_absent':True,'manager_stop':stopped};publish(case/'proof.json',row);rows.append(row)
  expected='completed' if scenario in ('absent','unregistered','stopped') else 'cancelled' if scenario=='cancel' else 'failed'
  if r['state']!=expected:raise ValueError('fixture_case_result_'+scenario)
 after=readback();publish(d/'after.private.json',after)
 if canonical(before)!=canonical(after):raise ValueError('preservation_changed')
 result={'schema':1,'source':manifest['source'],'seal':seal,'sessions':rows,'preservation':{'unchanged':True,'system':after['system'],'capture':after['capture'],'retained':len(after['retained']),'projects':len(after['projects'])},'real_dependency_mutations':0,'commercial_launch':False}
 publish(d/'proof.json',result)
if __name__=='__main__':
 import sys
 run(*sys.argv[1:])

"""Integrated sealed application/dependency campaign, source-owned only."""
import hashlib,json,os,pathlib,shutil,subprocess,time
from owner_package import verify,digest,read,publish,canonical
from owner_campaign import preservation,compare_preservation
from application_supervise import CASES
def lifetime_sample(report,op):
 import ownership
 r=read(report);ledger=read(report.parent/(op+'-ledger.private.json'))['ledger']
 if r['state']!='running' or not r['dependency']['ready_tested'] or r['effective'] is None or r['renderer']['launch_binding']['status']!='bound':raise ValueError('lifetime_not_running')
 if ledger['dropped_process_observations']:raise ValueError('lifetime_ledger_loss')
 identities={}
 for phase in ('dependency_start','application_runner'):
  rows=[x for x in ledger['processes'] if x['phase']==phase and x['relationship']=='direct_launcher']
  if len(rows)!=1:raise ValueError('lifetime_launcher_ambiguity')
  row=rows[0];pid=row['pid'];proc=pathlib.Path('/proc')/str(pid)
  stat=ownership.bounded(proc/'stat','stat');start,_=ownership.generation(stat,pid)
  state=stat.rsplit(b') ',1)[1].split()[0]
  group=ownership.bounded(proc/'cgroup','stat').decode().strip()
  again,_=ownership.generation(ownership.bounded(proc/'stat','stat'),pid)
  if start!=row['start_ticks'] or again!=start or state==b'Z' or row['linux_exit'] is not None or group!='0::'+row['cgroup'] or not row['cgroup'].endswith('/linux-vst-bridge-renderer-'+op+'.service'):raise ValueError('lifetime_generation')
  identities[phase]=hashlib.sha256(canonical({'pid':pid,'start_ticks':start,'cgroup':row['cgroup']})).hexdigest()
 return {'generations':identities,'result_sha256':digest(report),'elapsed_ns':time.monotonic_ns()}
def run(package,seal,out,*,cases=CASES):
 p=pathlib.Path(package).resolve();manifest=verify(p,seal);d=pathlib.Path(out).resolve();os.umask(0o077);d.mkdir(mode=0o700)
 before=preservation();publish(d/'before.private.json',before)
 runners={canonical(v['registration']['environment']['runner']):v['registration']['environment'] for v in before['registry']['classes'].values() if v['registration']['environment']['runner']['id']=='proton-11.0-2c-25118279-slr4-4.0.20260805.254769'}
 if len(runners)!=1:raise ValueError('runner_ambiguous')
 template=next(iter(runners.values()));rows=[]
 for scenario in cases:
  verify(p,seal);case=d/scenario;case.mkdir(mode=0o700);root=(d/'normal' if scenario=='repeat' else case)/'environment'
  if scenario!='repeat':root.mkdir(mode=0o700)
  if scenario!='repeat':
   for name in ('home','compatdata','runtime-var','host-cache','host-config','host-data','host-tmp','client'):(root/name).mkdir(mode=0o700)
  (root/'operation.lock').touch(mode=0o600);op=os.urandom(16).hex();env=read(root/'environment.json') if scenario=='repeat' else dict(template,id=op,root=str(root),revision=1)
  if scenario!='repeat':publish(root/'environment.json',env)
  publish(case/'case.json',{'case':scenario});appfile=case/'application.exe';shutil.copyfile(p/'application.exe',appfile);appfile.chmod(0o400)
  install=case/'fixture-installation.json';publish(install,{'generated':True})
  app={'schema':1,'id':'nad1-source-owned','environment':env,'files':{'Native Access.exe':{'artifact':{'path':str(appfile),'sha256':digest(appfile)},'size':appfile.stat().st_size}},'installation':{'path':str(install),'sha256':digest(install)},'observation_sha256':'0'*64,'source_seal_sha256':seal}
  sw=dict(before['software'])
  for k,n in [('manager','binding-owner'),('supervisor','session.py'),('ownership','ownership.py'),('installer_launch','adapter.exe')]:sw[k]={'path':str(p/n),'sha256':manifest['files'][n]}
  publish(case/'app.private.json',app);publish(case/'software.private.json',sw);manager=case/'manager';report=manager/'vendor-applications/native-access/operations'/op/'result.json';spec=case/'spec.private.json'
  subprocess.run([str(p/'binding-owner'),str(case/'app.private.json'),str(case/'software.private.json'),op,'software_rendering',str(report),str(spec),'application'],check=True,timeout=20)
  # Production renderer reservation/Stop owner, sealed generated entry only.
  subprocess.run([str(p/'binding-owner'),'application-submit',str(manager),str(spec),str(p),seal],check=True,timeout=30)
  unit='linux-vst-bridge-renderer-'+op+'.service';deadline=time.monotonic()+320;stopped=False;held_since=None;lifetime=[]
  try:
   while time.monotonic()<deadline:
    if report.exists() and read(report).get('cleanup_confirmed'):break
    if scenario=='lifetime' and (root/'compatdata/pfx/drive_c/NAD1Fixture/application-started').exists() and report.exists() and read(report).get('effective'):
     if held_since is None:
      lifetime.append(lifetime_sample(report,op));held_since=time.monotonic()
     elif len(lifetime)==1 and time.monotonic()-held_since>=12:
      lifetime.append(lifetime_sample(report,op))
      if lifetime[0]['generations']!=lifetime[1]['generations']:raise ValueError('lifetime_generation_changed')
      (root/'compatdata/pfx/drive_c/NAD1Fixture/explicit-exit').touch(mode=0o600)
    if scenario=='cancel' and (root/'compatdata/pfx/drive_c/NAD1Fixture/application-started').exists() and not stopped:
     subprocess.run([str(p/'binding-owner'),'application-stop',str(manager),op],check=True,timeout=120);stopped=True
    time.sleep(.2)
  finally:
   subprocess.run([str(p/'binding-owner'),'application-stop',str(manager),op],check=True,timeout=120)
  r=read(report) if report.exists() else read(report.parent/'recovery-result.json')
  if not r.get('cleanup_confirmed') or r.get('owned_live')!=0:raise ValueError('fixture_cleanup')
  state=subprocess.run(['systemctl','--user','show',unit,'--property=LoadState,ActiveState,MainPID,ControlPID,ControlGroup'],capture_output=True,timeout=15).stdout.decode()
  if dict(x.split('=',1) for x in state.splitlines())!={'LoadState':'not-found','ActiveState':'inactive','MainPID':'0','ControlPID':'0','ControlGroup':''}:raise ValueError('unit_not_absent')
  events=root/'compatdata/pfx/drive_c/NAD1Fixture/events.private'
  if events.exists():shutil.copyfile(events,case/'events.private')
  launched=(root/'compatdata/pfx/drive_c/NAD1Fixture/application-started').exists()
  if launched!=(scenario!='not_ready'):raise ValueError('fixture_application_gate')
  dep=r['dependency']
  if dep['service_retirement_confirmed']!=(scenario!='stop_failure') or dep['forced_cleanup_used']!=(scenario=='stop_failure') or not dep['process_cleanup_confirmed'] or not dep['service_stop_requested']:raise ValueError('fixture_retirement_result')
  actions=[x['action'] for x in dep['stages']]
  if actions.count('stop')!=1 or actions.count('start')!=1 or 'install' in actions:raise ValueError('fixture_no_duplicate_transition')
  if scenario!='not_ready' and (r.get('effective') is None or r['renderer']['launch_binding']['status']!='bound'):raise ValueError('fixture_application_root')
  if scenario=='stop_failure' and r['error']!='application_outer_nonzero':raise ValueError('first_failure_replaced')
  if scenario!='normal':shutil.rmtree(root)
  row={'case':scenario,'operation':op,'result':r,'result_sha256':digest(report if report.exists() else report.parent/'recovery-result.json'),'application_launched':launched,'prefix_removed':scenario!='normal','unit_absent':True,'manager_stop':stopped,'lifetime_samples':lifetime};publish(case/'proof.json',row);rows.append(row)
  expected='completed' if scenario in ('normal','repeat','lifetime') else 'cancelled' if scenario=='cancel' else 'failed'
  if scenario=='lifetime' and (len(lifetime)!=2 or lifetime[1]['elapsed_ns']-lifetime[0]['elapsed_ns']<12_000_000_000 or stopped):raise ValueError('lifetime_interval')
  if r['state']!=expected:raise ValueError('fixture_case_result_'+scenario)
 if (d/'normal/environment').exists():raise ValueError('repeat_prefix_not_removed')
 after=preservation();publish(d/'after.private.json',after)
 preservation_result=compare_preservation(before,after)
 result={'schema':1,'source':manifest['source'],'seal':seal,'runner_sha256':hashlib.sha256(canonical(template['runner'])).hexdigest(),'installed_software_sha256':hashlib.sha256(canonical(before['software'])).hexdigest(),'sessions':rows,'preservation':{'protected_state_unchanged':True,**preservation_result,'system':after['system'],'capture':after['capture'],'retained':len(after['retained']),'projects':len(after['projects']),'real_prefix_entries':after['real_prefix_entries'],'real_prefix_metadata_sha256':after['real_prefix_metadata_sha256'],'renderer_records_sha256':hashlib.sha256(canonical(after['renderer_records'])).hexdigest()},'real_dependency_mutations':0,'commercial_launch':False}
 publish(d/'proof.json',result)
if __name__=='__main__':
 import sys
 run(*sys.argv[1:])

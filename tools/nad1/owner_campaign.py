"""Generated installation/SCM lifecycle; no Native Access or real daemon input."""
import hashlib,json,os,pathlib,shutil,subprocess,time,stat
from owner_package import verify,digest,read,publish,canonical

def preservation():
 from readback import readback
 value=readback();root=pathlib.Path.home()/'.local/share/linux-vst-bridge/managed/environments/627d2cba97edbecf113c22504eb4c81b'
 pending=[root];rows={};deadline=time.monotonic()+60
 while pending:
  directory=pending.pop()
  with os.scandir(directory) as entries:
   for entry in entries:
    if len(rows)>=100000 or time.monotonic()>deadline:raise ValueError('preservation_extent')
    s=entry.stat(follow_symlinks=False);path=directory/entry.name
    rows[str(path.relative_to(root))]=[s.st_mode,s.st_size,s.st_mtime_ns,s.st_ctime_ns,s.st_dev,s.st_ino]
    if stat.S_ISDIR(s.st_mode):pending.append(path)
 value['real_prefix_metadata_sha256']=hashlib.sha256(canonical(rows)).hexdigest();value['real_prefix_entries']=len(rows)
 value['environment_metadata']=rows;runtime={};total=0
 for name,row in rows.items():
  if name.startswith('runtime-var/') and stat.S_ISREG(row[0]):
   total+=row[1]
   if total>4*1024**3 or row[1]>512*1024**2:raise ValueError('runtime_content_extent')
   file=root/name;before=file.stat();h=digest(file);after=file.stat()
   if (before.st_dev,before.st_ino,before.st_size,before.st_mtime_ns)!=(after.st_dev,after.st_ino,after.st_size,after.st_mtime_ns):raise ValueError('runtime_content_changed')
   runtime[name]={'sha256':h,'links':after.st_nlink}
 value['runtime_content']=runtime
 managed=root.parents[1]
 value['renderer_records']={str(f.relative_to(managed)):digest(f) for f in (managed/'vendor-applications/native-access').rglob('*.json') if f.is_file()}
 return value

def compare_preservation(before,after):
 allowed={'environment_metadata','real_prefix_metadata_sha256','runtime_content'}
 if {k:v for k,v in before.items() if k not in allowed}!={k:v for k,v in after.items() if k not in allowed}:raise ValueError('preservation_changed')
 a=before['environment_metadata'];b=after['environment_metadata'];count=0
 if set(a)!=set(b) or before['runtime_content']!=after['runtime_content']:raise ValueError('preservation_content_changed')
 for name in a:
  if a[name]==b[name]:continue
  if not name.startswith('runtime-var/') or not stat.S_ISREG(a[name][0]) or before['runtime_content'].get(name,{}).get('links',0)<2 or any(a[name][i]!=b[name][i] for i in (0,1,2,4,5)):raise ValueError('preservation_metadata_changed')
  count+=1
 return {'runtime_shared_inode_ctime_changes':count,'runtime_content_unchanged':True,'windows_prefix_metadata_unchanged':True,'private_home_metadata_unchanged':True}

def run(package,seal,out,*,cases=('absent','unregistered','stopped','ready','not_ready','cancel')):
 p=pathlib.Path(package).resolve();manifest=verify(p,seal);d=pathlib.Path(out).resolve();os.umask(0o077);d.mkdir(mode=0o700)
 before=preservation();publish(d/'before.private.json',before)
 runners={canonical(v['registration']['environment']['runner']):v['registration']['environment'] for v in before['registry']['classes'].values() if v['registration']['environment']['runner']['id']=='proton-11.0-2c-25118279-slr4-4.0.20260805.254769'}
 if len(runners)!=1:raise ValueError('runner_ambiguous')
 template=next(iter(runners.values()));rows=[]
 for scenario in cases:
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
  if scenario.startswith('recovery'):
   value=read(spec);value['dependency_mode']='recover_installed'
   # Source-owned fixture mode is fixed by this sealed campaign, before reservation.
   spec.unlink();publish(spec,value)
  # The shared manager example expects supervise.py; the package exposes only the
  # fixed sealed owner_supervise.py, selected by this source-owned Rust build.
  subprocess.run([str(p/'binding-owner'),'submit',str(manager),str(spec),str(p),seal],check=True,timeout=30)
  unit='linux-vst-bridge-dependency-'+op+'.service';deadline=time.monotonic()+320;stopped=False
  try:
   while time.monotonic()<deadline:
    if report.exists():break
    if scenario in ('cancel','recovery_cancel') and list(report.parent.glob(op+'-dependency-stage-1.json')) and not stopped:
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
  expected='completed' if scenario in ('absent','unregistered','stopped','ready','recovery') else 'cancelled' if scenario in ('cancel','recovery_cancel') else 'failed'
  if r['state']!=expected:raise ValueError('fixture_case_result_'+scenario)
  if scenario.startswith('recovery'):
   dependency=r['dependency'];stages=dependency['stages']
   if not dependency.get('recovery') or any(x['action']=='install' for x in stages):raise ValueError('fixture_recovery_reinstalled')
   if sum(x['action']=='stop' for x in stages)!=1 or not dependency['service_retirement_confirmed'] or dependency['forced_cleanup_used']:raise ValueError('fixture_recovery_retirement')
   q=read(case/'recovery-qualification.json')
   if any(digest(case/'failed-installation'/name)!=v['sha256'] for name,v in q['sources'].items()):raise ValueError('fixture_prior_failure_changed')
   if dependency['ready_tested']!=(scenario=='recovery'):raise ValueError('fixture_recovery_readiness')
 after=preservation();publish(d/'after.private.json',after)
 preservation_result=compare_preservation(before,after)
 result={'schema':1,'source':manifest['source'],'seal':seal,'runner_sha256':hashlib.sha256(canonical(template['runner'])).hexdigest(),'installed_software_sha256':hashlib.sha256(canonical(before['software'])).hexdigest(),'sessions':rows,'preservation':{'protected_state_unchanged':True,**preservation_result,'system':after['system'],'capture':after['capture'],'retained':len(after['retained']),'projects':len(after['projects']),'real_prefix_entries':after['real_prefix_entries'],'real_prefix_metadata_sha256':after['real_prefix_metadata_sha256'],'renderer_records_sha256':hashlib.sha256(canonical(after['renderer_records'])).hexdigest()},'real_dependency_mutations':0,'commercial_launch':False}
 publish(d/'proof.json',result)
if __name__=='__main__':
 import sys
 run(*sys.argv[1:])

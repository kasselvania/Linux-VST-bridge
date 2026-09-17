"""Disposable generated prefix, exact pinned runner and production cgroup ledger."""
import ctypes,hashlib,json,os,pathlib,shutil,subprocess,sys,time
sys.dont_write_bytecode=True
from lead_package import canonical,digest,publish,verify

def child(package,seal,out):
 m=verify(package,seal);d=pathlib.Path(out);spec=json.loads((d/'input.private.json').read_bytes())
 if d.parent!=pathlib.Path.home()/'.cache/linux-vst-bridge' or not d.name.startswith('nad1-lead-'):raise ValueError('fixture_root')
 op=spec['operation'];root=d/'environment';env=spec['environment']
 if env['root']!=str(root) or env['id']!=op:raise ValueError('fixture_environment')
 if digest(d/'NTKDaemon.exe')!=m['files']['NTKDaemon.exe']:raise ValueError('fixture_image')
 import session
 from dependency_process import census,image_identity
 scope=session.CompanionCgroup(installer_operation=op)
 if scope.members():raise ValueError('occupied_fixture_unit')
 if ctypes.CDLL(None,use_errno=True).prctl(36,1,0,0,0)!=0:raise RuntimeError('subreaper')
 ledger=session.InstallerLedger(scope,lambda v:session.installer_atomic(d/'ledger.private.json',v))
 runner=env['runner']
 for a in runner['files']:
  if digest(a['path'])!=a['sha256']:raise ValueError('runner_changed')
 environ=session.environment({'environment':env,'compatibility':{'disable_windows_accessibility':False}});environ['HOME']=str(root/'home')
 def run(args,phase,observe=False):
  process=subprocess.Popen([runner['entry_point'],'--verb=run','--',runner['proton'],*args],env=environ,stdin=subprocess.DEVNULL,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,start_new_session=True)
  ledger.launcher(process,phase);deadline=time.monotonic()+120;admitted=image_identity(d/'NTKDaemon.exe');found=None
  while process.returncode is None and time.monotonic()<deadline:
   ledger.harvest()
   if observe and found is None:
    value=census(root/'compatdata/pfx',admitted)
    matches=[x for x in value['candidates'] if x['exact'] and x['prefix_relation']=='same']
    if len(matches)==1:
     found=value;publish(d/'census.private.json',value)
   time.sleep(.1)
  code=process.returncode;clean=ledger.cleanup()
  if code!=0 or not clean:raise ValueError('fixture_process_or_cleanup')
  return found
 try:
  run(['getcompatpath','/'],'fixture_prefix_initialization')
  found=run(['runinprefix',str(d/'NTKDaemon.exe')],'fixture_lead',True)
  if found is None:raise ValueError('pinned_runner_lead_not_established')
  found.pop('private')
  publish(d/'child-result.json',{'schema':1,'seal':seal,'source':m['source'],'operation':op,'runner_sha256':hashlib.sha256(canonical(runner)).hexdigest(),'payload':image_identity(d/'NTKDaemon.exe'),'census':found,'cleanup_confirmed':True,'owned_live':len(scope.members()),'linux_windows_join':False})
 finally:
  if not ledger.cleanup():raise RuntimeError('fixture_cleanup_unconfirmed')

def campaign(package,seal,out):
 package=pathlib.Path(package).resolve();m=verify(package,seal);os.umask(0o077);d=pathlib.Path(out)
 # Reuse the already retained NAD1 input record. No discovery of commercial files.
 cache=pathlib.Path.home()/'.cache/linux-vst-bridge/nad1-observation-4d0121697631/private.json'
 expected='bb22c25bed3cea798b0ed75e35084c9668ee95889e22c615d8c9f104366db2d3'
 if digest(cache)!=expected:raise ValueError('retained_input_changed')
 context=json.loads(cache.read_bytes());software=context['software'];manager=software['manager']
 if digest(manager['path'])!=manager['sha256']:raise ValueError('installed_manager_changed')
 def idle():
  r=json.loads(subprocess.check_output([manager['path'],'operator','activity'],timeout=90));s=r['system']
  if s['service']!='active' or s['keepers']!=2 or s['cleanup_unconfirmed'] or any(s[k] for k in ('dsp','maintenance','pending_transactions','stale_transports')) or r['capture']['armed']:raise ValueError('not_idle')
  return r
 before=idle();d.mkdir(mode=0o700);root=d/'environment';root.mkdir(mode=0o700)
 for name in ('home','compatdata','runtime-var','host-cache','host-config','host-data','host-tmp','client'):(root/name).mkdir(mode=0o700)
 (root/'operation.lock').touch(mode=0o600);op=os.urandom(16).hex();env=dict(context['application']['environment'],id=op,root=str(root),revision=1)
 publish(d/'input.private.json',{'operation':op,'environment':env});shutil.copyfile(package/'NTKDaemon.exe',d/'NTKDaemon.exe');(d/'NTKDaemon.exe').chmod(0o400)
 unit='linux-vst-bridge-installer-'+op+'.service'
 submitted=subprocess.run(['systemd-run','--user','--collect','--property=UMask=0077','--property=KillMode=control-group','--property=TimeoutStopSec=20','--property=StandardOutput=null','--property=StandardError=null','--unit='+unit,'/usr/bin/python3','-B',str(package/'lead_run.py'),'child',str(package),seal,str(d)],timeout=20)
 deadline=time.monotonic()+180
 try:
  while time.monotonic()<deadline:
   if (d/'child-result.json').exists():break
   if subprocess.run(['systemctl','--user','is-active','--quiet',unit]).returncode!=0:break
   time.sleep(.25)
 finally:
  subprocess.run(['systemctl','--user','stop',unit],timeout=35,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
 fields=subprocess.check_output(['systemctl','--user','show',unit,'--property=LoadState,ActiveState,MainPID,ControlPID,ControlGroup'],timeout=15).decode()
 expected_fields={'LoadState':'not-found','ActiveState':'inactive','MainPID':'0','ControlPID':'0','ControlGroup':''}
 if dict(x.split('=',1) for x in fields.splitlines())!=expected_fields:raise ValueError('fixture_unit_not_retired')
 if not (d/'child-result.json').exists():raise ValueError('fixture_result_missing_retired')
 result=json.loads((d/'child-result.json').read_bytes())
 if not result['cleanup_confirmed'] or result['owned_live']:raise ValueError('fixture_cleanup')
 shutil.rmtree(root);after=idle()
 if before!=after:raise ValueError('idle_state_changed')
 result.update(prefix_removed=True,unit_absent=True,cgroup_absent=True,preservation=after,submission_exit=submitted.returncode,commercial_launch=False)
 publish(d/'proof.json',result);print(canonical({'result_sha256':digest(d/'proof.json'),'lead_qualified':True}).decode())
if __name__=='__main__':
 if len(sys.argv)!=5:raise ValueError('fixture_arguments')
 {'child':child,'campaign':campaign}[sys.argv[1]](*sys.argv[2:])

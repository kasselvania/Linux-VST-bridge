"""Sealed disposable application + SCM fixture; never installed admission."""
import ctypes,os,pathlib,shutil,subprocess,sys,time
sys.dont_write_bytecode=True
from owner_package import verify,read,digest,publish
CASES=('normal','repeat','registration_handoff','registration_absent','cancel','not_ready','stop_failure','application_failure','lifetime','child_memory','callback',
 'no_transition_cleanup','not_submitted_cleanup','stopped_residue_cleanup','observation_unavailable_cleanup')
def recovery_session(package,manifest,spec,d,fixture_directory):
 """Cross Rust and Python admission with an exact artifact-absent recovery origin."""
 import session as s
 recovery=d/'session-recovery';operation=os.urandom(16).hex();prior=recovery/'operations'/operation
 prior.mkdir(parents=True,mode=0o700)
 historical=dict(spec,schema=2,kind='native_access_dependency',operation=operation)
 publish(prior/'spec.json',historical)
 publish(prior/'result.json',{'operation':operation,'state':'failed','cleanup_confirmed':True,'owned_live':0,
  'dependency':{'service_retirement_confirmed':True,'process_cleanup_confirmed':True,'forced_cleanup_used':False}})
 log=prior/(operation+'-dependency-1.log')
 with log.open('xb') as f:f.write(b'source-owned retained dependency failure');f.flush();os.fsync(f.fileno());os.fchmod(f.fileno(),0o400)
 sources={path.name:{'sha256':digest(path),'size':path.stat().st_size} for path in prior.iterdir()}
 daemon=__import__('ownership').image_identity(fixture_directory/'NTKDaemon.exe')
 q={'schema':1,'operation':operation,'application_identity':spec['application_identity'],
  'installer_sha256':manifest['files']['Setup.exe'],'installer_size':(package/'Setup.exe').stat().st_size,
  'daemon':daemon,'sources':sources}
 qualification=d/'recovery-qualification.private.json';publish(qualification,q);session_path=d/'dependency-session.private.json'
 subprocess.run([str(package/'binding-owner'),'application-session',str(d/'app.private.json'),str(d/'software.private.json'),
  str(recovery),str(qualification),str(session_path)],check=True,timeout=30)
 session=read(session_path);spec['dependency_session']=session;s.renderer_bound_inputs(spec)
 if s.nad1_session_admitted_inputs(spec,recovery,q,'NAD1Fixture/Setup.exe','NAD1Fixture/NTKDaemon.exe')!=session:raise ValueError('fixture_session_admission_disagreement')
 if (recovery/'artifact.json').exists() or (recovery/'prepared.json').exists():raise ValueError('fixture_recovery_pointer_created')
 return session,recovery,q
def run(package,seal,spec_path):
 p=pathlib.Path(package);manifest=verify(p,seal);spec=read(spec_path);d=pathlib.Path(spec_path).parents[5];op=spec['operation']
 case=read(d/'case.json')['case']
 if case not in CASES or d.name!=case:raise ValueError('fixture_case')
 root=(d.parent/'normal' if case=='repeat' else d)/'environment'
 report=d/'manager/vendor-applications/native-access/operations'/op/'result.json'
 if spec['kind']!='renderer_application' or spec['renderer_policy']!='software_rendering' or spec['report']!=str(report) or pathlib.Path(spec_path)!=report.parent/'spec.json':raise ValueError('fixture_report')
 app=spec['application'];file=d/'application.exe';installation=d/'fixture-installation.json'
 if app['id']!='nad1-source-owned' or app['source_seal_sha256']!=seal or app['observation_sha256']!='0'*64 or app['environment']['root']!=str(root):raise ValueError('fixture_binding')
 if app['files']!={'Native Access.exe':{'artifact':{'path':str(file),'sha256':manifest['files']['application.exe']},'size':(p/'application.exe').stat().st_size}} or digest(file)!=manifest['files']['application.exe']:raise ValueError('fixture_image')
 if app['installation']!={'path':str(installation),'sha256':digest(installation)} or read(installation)!={'generated':True}:raise ValueError('fixture_installation')
 if read(report.parents[2]/'current.json')!={'operation':op,'application':spec['application_identity'],'policy':'software_rendering'}:raise ValueError('fixture_reservation')
 for f in [d,root,file,installation,pathlib.Path(spec_path)]:
  if f.resolve()!=f:raise ValueError('fixture_alias')
 for field,name in [('manager','binding-owner'),('supervisor','session.py'),('ownership','ownership.py'),('installer_launch','adapter.exe')]:
  if spec['software'][field]!={'path':str(p/name),'sha256':manifest['files'][name]}:raise ValueError('fixture_owner')
 import session as s,ownership
 s.renderer_bound_inputs(spec)
 if ctypes.CDLL(None,use_errno=True).prctl(36,1,0,0,0)!=0:raise RuntimeError('subreaper')
 scope=s.CompanionCgroup(renderer_operation=op);ledger=s.InstallerLedger(scope,lambda v:s.installer_atomic(d/'initialization.private.json',v))
 fixture={'installer':'NAD1Fixture/Setup.exe','daemon':'NAD1Fixture/NTKDaemon.exe','installer_sha256':manifest['files']['Setup.exe'],'installer_size':(p/'Setup.exe').stat().st_size}
 directory=root/'compatdata/pfx/drive_c/NAD1Fixture'
 if case!='repeat':
  env=app['environment'];runner=env['runner'];launch=s.environment({'environment':env,'compatibility':{'disable_windows_accessibility':False}});launch['HOME']=str(root/'home')
  child=subprocess.Popen([runner['entry_point'],'--verb=run','--',runner['proton'],'getcompatpath','/'],env=launch,stdin=subprocess.DEVNULL,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,start_new_session=True)
  ledger.launcher(child,'fixture_initialization');deadline=time.monotonic()+120
  while child.returncode is None and time.monotonic()<deadline:ledger.harvest();time.sleep(.05)
  code=child.returncode
  if not ledger.cleanup() or code!=0:raise ValueError('fixture_initialization')
  directory.mkdir();shutil.copyfile(p/'Setup.exe',directory/'Setup.exe')
  if case=='child_memory':
   # Cold same-prefix control before any fixture service is installed or started.
   argv=[runner['entry_point'],'--verb=run','--',runner['proton'],'runinprefix',str(file),'--memory-control']
   with (d/'memory-control.private.log').open('xb') as log:
    child=subprocess.Popen(argv,env=launch,cwd=root/'home',stdin=subprocess.DEVNULL,stdout=log,stderr=log,start_new_session=True)
    ledger.launcher(child,'memory_control');deadline=time.monotonic()+60
    while child.returncode is None and time.monotonic()<deadline:ledger.harvest();time.sleep(.05)
    if not ledger.cleanup():raise ValueError('memory_control_cleanup')
   if not (directory/'memory-control.json').exists():raise ValueError('memory_control_missing')
  seed=d/'seed';seed.mkdir(mode=0o700);seeder=s.Nad1Owner(dict(spec,report=str(seed/'result.json')),ledger,scope,lambda:False,fixture=fixture)
  error=None
  try:seeder.ensure(True)
  except Exception as exc:error=type(exc).__name__;raise
  finally:
   retired=seeder.retire();clean=ledger.cleanup();seeder.process_cleanup_confirmed=clean
   publish(seed/'result.json',{'dependency':seeder.value(),'error':error,'cleanup_confirmed':clean})
   if not retired or not clean:raise ValueError('fixture_preparation_retirement')
 else:
  previous=read(d.parent/'normal/proof.json')['result']
  if previous['state']!='completed' or not previous['dependency']['service_retirement_confirmed'] or previous['dependency']['forced_cleanup_used']:raise ValueError('fixture_repeat_prior_retirement')
  (directory/'application-started').unlink()
 session,recovery,qualification=recovery_session(p,manifest,spec,d,directory)
 fixture.update(session_directory=str(recovery),session_qualification=qualification)
 if case=='registration_handoff':(directory/'query-absent-once').touch()
 if case=='registration_absent':(directory/'query-absent-always').touch()
 if case=='callback':(directory/'await-callback').touch()
 if case=='child_memory':(directory/'probe-child-memory').touch()
 if case=='not_ready':(directory/'not-ready').touch()
 if case=='lifetime':(directory/'await-explicit-exit').touch()
 if case=='cancel':(directory/'hold-application').touch()
 if case in ('application_failure','stop_failure'):(directory/'fail-application').touch()
 if case=='stop_failure':(directory/'refuse-stop').touch()
 if case=='no_transition_cleanup':(directory/'no-transition').touch()
 if case=='not_submitted_cleanup':(directory/'control-failure').touch()
 if case=='stopped_residue_cleanup':(directory/'stopped-listener-hold').touch()
 if case=='observation_unavailable_cleanup':(directory/'query-failure').touch()
 admitted=session['daemon']
 if admitted['sha256']!=manifest['files']['Setup.exe']:raise ValueError('fixture_daemon_changed')
 verify(p,seal)
 return s.renderer_owned(spec,dependency=session,dependency_fixture=fixture,callback_fixture=case=='callback')
if __name__=='__main__':
 if len(sys.argv)!=3:raise ValueError('fixture_arguments')
 sys.exit(0 if run(pathlib.Path(__file__).parent,sys.argv[1],sys.argv[2]) else 1)

"""Sealed generated service entry; not an installed command or persisted bypass."""
import ctypes,json,os,pathlib,shutil,subprocess,sys,time
sys.dont_write_bytecode=True
from owner_package import verify,read,digest,publish

def run(package,seal,spec_path):
 p=pathlib.Path(package);manifest=verify(p,seal);spec=read(spec_path);d=pathlib.Path(spec_path).parents[5];root=d/'environment';op=spec['operation']
 expected_report=d/'manager/vendor-applications/native-access-dependency/operations'/op/'result.json'
 if spec['report']!=str(expected_report) or spec['application']['environment']['root']!=str(root) or spec['application']['id']!='nad1-source-owned' or spec['application']['source_seal_sha256']!=seal:raise ValueError('fixture_binding')

 if pathlib.Path(spec_path)!=expected_report.parent/'spec.json' or spec['kind']!='native_access_dependency' or spec['renderer_policy']!='software_rendering':raise ValueError('fixture_spec_location')
 if read(expected_report.parents[2]/'current.json')!={'operation':op,'application':spec['application_identity'],'policy':'software_rendering'}:raise ValueError('fixture_reservation')
 app=spec['application'];file=d/'source-owned.exe';installation=d/'fixture-installation.json'
 if app['files']!={'Native Access.exe':{'artifact':{'path':str(file),'sha256':manifest['files']['Setup.exe']},'size':(p/'Setup.exe').stat().st_size}} or digest(file)!=manifest['files']['Setup.exe']:raise ValueError('fixture_image')
 if app['installation']!={'path':str(installation),'sha256':digest(installation)} or read(installation)!={'generated':True} or app['observation_sha256']!='0'*64:raise ValueError('fixture_installation')
 for path in [d,root,file,installation,pathlib.Path(spec_path)]:
  if path.resolve()!=path:raise ValueError('fixture_alias')
 scenario=read(d/'case.json')['case']
 if scenario not in ('absent','unregistered','stopped','ready','not_ready','cancel','recovery','recovery_not_ready','recovery_cancel'):raise ValueError('fixture_case')
 for field,name in [('manager','binding-owner'),('supervisor','session.py'),('ownership','ownership.py'),('installer_launch','adapter.exe')]:
  if spec['software'][field]!={'path':str(p/name),'sha256':manifest['files'][name]}:raise ValueError('fixture_owner')
 import session as s,ownership
 if spec.get('schema')!=2 or spec.get('dependency_mode')!=('recover_installed' if scenario.startswith('recovery') else 'prepare'):raise ValueError('fixture_dependency_mode')
 s.renderer_bound_inputs(dict({k:v for k,v in spec.items() if k!='dependency_mode'},schema=1,kind='renderer_application'))
 if ctypes.CDLL(None,use_errno=True).prctl(36,1,0,0,0)!=0:raise RuntimeError('subreaper')
 scope=s.CompanionCgroup(dependency_operation=op);ledger=s.InstallerLedger(scope,lambda v:s.installer_atomic(d/'initialization.private.json',v))
 env=spec['application']['environment'];runner=env['runner'];launch=s.environment({'environment':env,'compatibility':{'disable_windows_accessibility':False}});launch['HOME']=str(root/'home')
 child=subprocess.Popen([runner['entry_point'],'--verb=run','--',runner['proton'],'getcompatpath','/'],env=launch,stdin=subprocess.DEVNULL,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,start_new_session=True);ledger.launcher(child,'fixture_initialization');deadline=time.monotonic()+120
 while child.returncode is None and time.monotonic()<deadline:ledger.harvest();time.sleep(.05)
 code=child.returncode
 if not ledger.cleanup() or code!=0:raise ValueError('fixture_initialization')
 directory=root/'compatdata/pfx/drive_c/NAD1Fixture';directory.mkdir();shutil.copyfile(p/'Setup.exe',directory/'Setup.exe')
 fixture={'installer':'NAD1Fixture/Setup.exe','daemon':'NAD1Fixture/NTKDaemon.exe','installer_sha256':manifest['files']['Setup.exe'],'installer_size':(p/'Setup.exe').stat().st_size}
 if scenario=='unregistered':shutil.copyfile(p/'Setup.exe',directory/'NTKDaemon.exe');fixture['admitted']=ownership.image_identity(directory/'NTKDaemon.exe')
 if scenario in ('stopped','ready','not_ready','cancel'):
  seed=d/'seed';seed.mkdir(mode=0o700);seed_spec=dict(spec,report=str(seed/'result.json'))
  seeder=s.Nad1Owner(seed_spec,ledger,scope,lambda:False,fixture=fixture)
  try:seeder.command('install')
  finally:
   if not ledger.cleanup():raise ValueError('fixture_seed_cleanup')
  fixture['admitted']=ownership.image_identity(directory/'NTKDaemon.exe')
 if scenario.startswith('recovery'):
  # Seed a genuinely failed source-owned installation, then recover its exact
  # payload through the same verifier/SCM owner. No production test-mode field.
  import hashlib
  seed=d/'failed-installation';seed.mkdir(mode=0o700)
  seed_op=hashlib.sha256((op+'seed').encode()).hexdigest()[:32]
  seed_spec=dict(spec,operation=seed_op,dependency_mode='prepare',report=str(seed/'result.json'))
  publish(seed/'spec.json',seed_spec)
  marker=directory/'fail-after-register';marker.touch()
  seeder=s.Nad1Owner(seed_spec,ledger,scope,lambda:False,fixture=fixture)
  error=None
  try:seeder.ensure(True)
  except ValueError as exc:error=str(exc)
  finally:
   retired=seeder.retire();clean=ledger.cleanup();seeder.process_cleanup_confirmed=clean
   publish(seed/'result.json',{'operation':seed_op,'state':'failed','error':error,'dependency':seeder.value(),'cleanup_confirmed':clean,'owned_live':0 if clean else None})
  if error!='dependency_installer_nonzero' or not retired or not clean:raise ValueError('fixture_failed_installation_seed')
  marker.unlink()
  names=['spec.json','result.json',seed_op+'-dependency-1.log']
  q={'schema':1,'operation':seed_op,'application_identity':spec['application_identity'],
     'installer_sha256':manifest['files']['Setup.exe'],'installer_size':(p/'Setup.exe').stat().st_size,
     'daemon':ownership.image_identity(directory/'NTKDaemon.exe'),
     'sources':{n:{'sha256':digest(seed/n),'size':(seed/n).stat().st_size} for n in names}}
  if q['daemon']['sha256']!=manifest['files']['Setup.exe']:raise ValueError('fixture_bundle_payload')
  publish(d/'recovery-qualification.json',q)
  fixture['recovery']=s.nad1_recovery_inputs(spec,seed,q,fixture['installer'],fixture['daemon'])
  if scenario in ('recovery_not_ready','recovery_cancel'):(directory/'not-ready').touch()
 if scenario=='ready':fixture['prestart']=True
 if scenario in ('not_ready','cancel'):(directory/'not-ready').touch()
 verify(p,seal)
 return s.nad1_owned(spec,fixture=fixture)
if __name__=='__main__':
 if len(sys.argv)!=3:raise ValueError('fixture_arguments')
 sys.exit(0 if run(pathlib.Path(__file__).parent,sys.argv[1],sys.argv[2]) else 1)

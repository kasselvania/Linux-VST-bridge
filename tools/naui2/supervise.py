"""Sealed fixture only; production application owner receives no test-mode field."""
import os,pathlib,subprocess,sys,time,ctypes
sys.dont_write_bytecode=True
from package import verify,read,digest,publish

def run(package,seal,spec_path):
    p=pathlib.Path(package);m=verify(p,seal);spec=read(spec_path)
    app=spec['application'];root=pathlib.Path(app['environment']['root']);op=spec['operation']
    if app['files']['Native Access.exe']['artifact']['sha256']!=m['files']['payload.exe'] or app['files']['Native Access.exe']['size']!=m['payload_size']:raise ValueError('not_source_owned_application')
    for name,file in [('manager','binding-owner'),('supervisor','session.py'),('ownership','ownership.py'),('installer_launch','adapter.exe')]:
        if spec['software'][name]!={'path':str(p/file),'sha256':m['files'][file]}:raise ValueError('fixture_owner_drift')
    import session
    # Fresh disposable prefix initialization, not a commercial app launch or
    # environment repair. Positive retirement before the production app owner.
    scope=session.CompanionCgroup(renderer_operation=op)
    if scope.members():raise ValueError('fixture_cgroup_occupied')
    if ctypes.CDLL(None,use_errno=True).prctl(36,1,0,0,0)!=0:raise RuntimeError('subreaper')
    ledger=session.InstallerLedger(scope,lambda v:session.installer_atomic(pathlib.Path(spec_path).with_name('initialization.private.json'),v))
    env=session.environment({'environment':app['environment'],'compatibility':{'disable_windows_accessibility':False}});env['HOME']=str(root/'home')
    runner=app['environment']['runner']
    child=subprocess.Popen([runner['entry_point'],'--verb=run','--',runner['proton'],'getcompatpath','/'],env=env,stdin=subprocess.DEVNULL,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,start_new_session=True)
    ledger.launcher(child,'fixture_prefix_initialization');deadline=time.monotonic()+120
    while child.returncode is None and time.monotonic()<deadline:ledger.harvest();time.sleep(.05)
    code=child.returncode;clean=ledger.cleanup()
    if code!=0 or not clean or not (root/'compatdata/pfx/system.reg').exists():raise ValueError('fixture_prefix_initialization_failed')
    verify(p,seal)
    return session.vendor_application(spec)
if __name__=='__main__':
    if len(sys.argv)!=3:raise ValueError('fixture_args')
    sys.exit(0 if run(pathlib.Path(__file__).parent,sys.argv[1],sys.argv[2]) else 1)

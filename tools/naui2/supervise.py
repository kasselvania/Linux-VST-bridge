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
    # This fixture authority is sealed and separate from installed Native Access
    # admission. No persisted flag can select it through --vendor-application.
    report=pathlib.Path(spec['report']);d=report.parents[5]
    expected_report=d/'manager/vendor-applications/native-access/operations'/op/'result.json'
    if report!=expected_report or root!=d/'environment' or app['environment']['id']!=op:raise ValueError('fixture_location')
    if app['id']!='naui2-source-owned' or app['observation_sha256']!='0'*64 or app['source_seal_sha256']!=seal:raise ValueError('fixture_provenance')
    expected_image={'artifact':{'path':str(d/'fixture.exe'),'sha256':m['files']['payload.exe']},'size':m['payload_size']}
    if app['files']!={'Native Access.exe':expected_image}:raise ValueError('fixture_image_set')
    if app['installation']!={'path':str(d/'fixture-installation.json'),'sha256':digest(d/'fixture-installation.json')}:raise ValueError('fixture_installation')
    context=read(p/'context.private.json');expected='proton-11.0-2c-25118279-slr4-4.0.20260805.254769'
    candidates={__import__('json').dumps(v['registration']['environment'],sort_keys=True):v['registration']['environment'] for v in context['registry']['classes'].values() if v['registration']['environment']['runner']['id']==expected}
    if not any(app['environment']==dict(v,id=op,root=str(root),revision=1) for v in candidates.values()):raise ValueError('fixture_environment')
    sw=dict(context['software'])
    for name,file in [('manager','binding-owner'),('supervisor','session.py'),('ownership','ownership.py'),('installer_launch','adapter.exe')]:sw[name]={'path':str(p/file),'sha256':m['files'][file]}
    if spec['software']!=sw:raise ValueError('fixture_software')
    session.renderer_bound_inputs(spec)
    def initialize():
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
    return session.renderer_owned(spec,preparation=initialize)
if __name__=='__main__':
    if len(sys.argv)!=3:raise ValueError('fixture_args')
    sys.exit(0 if run(pathlib.Path(__file__).parent,sys.argv[1],sys.argv[2]) else 1)

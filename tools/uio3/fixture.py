"""Account-free private compositor/pinned-runner fixture. Never the operator display."""
import json
import os
from pathlib import Path
import sys
HERE=Path(__file__).resolve().parent
sys.path[:0]=[str(HERE.parent/'uio1'),str(HERE.parent/'uir1'),str(HERE.parent.parent/'bridge-manager/runtime')]
from isolation import compositor_environment,runner_environment
from launch import Helper,sealed_bytes,private_json
# Load this owner by path: UIR1 also has a module named session.
import importlib.util
sys.path.insert(0,str(HERE))
_spec=importlib.util.spec_from_file_location('uio3_session',HERE/'session.py')
_session=importlib.util.module_from_spec(_spec);_spec.loader.exec_module(_session)
finish=_session.finish
import ownership

def inner(root,package):
    env=runner_environment(root,os.environ);runner=json.loads((root/'runner.json').read_text())
    files=json.loads((HERE/'package.json').read_text())['files']
    target=root/'compatdata/pfx/drive_c/uio3';target.mkdir(parents=True,mode=0o700)
    for name,sha in files.items():
        with (target/name).open('xb') as f:f.write(sealed_bytes(package/name,sha))
        (target/name).chmod(0o500)
    base=[runner['entry_point'],'--verb=run','--',runner['proton']]
    report=dict(schema=1,completed=False);handles=[]
    def run(command,label,seconds):
        h=Helper(ownership,command,env,target,root/(label+'.log'),seconds);handles.append(h)
        report[label]=finish(h)
    try:
        run([*base,'getcompatpath',str(target)],'initialize',60)
        run([*base,'runinprefix','C:\\uio3\\uio1-tests.exe','--input'],'windows_input',30)
        e=dict(env,UIO3_XVFB_FIXTURE='1')
        h=Helper(ownership,['/usr/bin/python3',str(HERE/'x11_fixture.py')],e,target,root/'xwayland.log',20);handles.append(h)
        report['xwayland']=finish(h);report['completed']=True
    finally:
        for h in handles:
            if not h.closed:
                try:finish(h)
                except Exception:report['completed']=False
        private_json(root/'fixture.json',report)
    return 0 if report['completed'] else 1

def outer(root,package,admission):
    root.mkdir(mode=0o700)
    if root.resolve()!=root:raise RuntimeError('fixture root alias')
    runner=json.loads(admission.read_text())['registration']['environment']['runner']
    if Path(runner['entry_point']).stat().st_dev!=root.stat().st_dev:raise RuntimeError('runtime/scratch filesystem mismatch')
    for a in runner['files']:sealed_runner(a)
    expected=json.loads((HERE/'package.json').read_text())['files']
    if set(expected)!={'uio1-observer.exe','uio1-hook.dll','uio1-accessibility.exe','uio1-tests.exe'}:raise RuntimeError('fixture helper set')
    for name,sha in expected.items():sealed_bytes(package/name,sha)
    private_json(root/'runner.json',runner)
    env=compositor_environment(root);env['UIO3_ROOT']=str(root);env['UIO3_PACKAGE']=str(package)
    command=['dbus-run-session','--','kwin_wayland','--virtual','--xwayland','--socket','uir1-isolated','--width','1280','--height','800','--no-lockscreen','--no-global-shortcuts','--no-kactivities','--exit-with-session',str(HERE/'fixture.sh')]
    r=finish(Helper(ownership,command,env,root,root/'compositor.log',150))
    public=dict(schema=1,compositor_exit=r['exit'],cleanup=r['cleanup'],fixture=json.loads((root/'fixture.json').read_text()))
    private_json(root/'result.json',public);print(json.dumps({'completed':public['fixture']['completed'],'compositor_exit':r['exit']}))

def sealed_runner(a):
    import hashlib
    with open(a['path'],'rb') as f:
        if hashlib.file_digest(f,'sha256').hexdigest()!=a['sha256']:raise RuntimeError('pinned runner bytes changed')

if __name__=='__main__':
    os.umask(0o077)
    if sys.argv[1:] == ['--inner']:sys.exit(inner(Path(os.environ['UIO3_ROOT']),Path(os.environ['UIO3_PACKAGE'])))
    if len(sys.argv)!=4:raise SystemExit('fixture.py NEW_PRIVATE_ROOT PACKAGE EXACT_ADMISSION_JSON')
    outer(*map(Path,sys.argv[1:]))

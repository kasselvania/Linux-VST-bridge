#!/usr/bin/env python3
"""One exact Blackhole vendor-editor graphics trace; private artifacts only."""
import argparse
import fcntl
import hashlib
import json
import os
import pathlib
import signal
import stat
import subprocess
import sys

CLASS='5653544248496D626C61636B686F6C65'
ENVIRONMENT='8b064373d533b72230cb4009952eaab4'
MODULE='b8a33c57b04eded38d0ca717bb5e3721330439f4931c401ea70e5246b516e90b'
HOST='5fa0907b045df5ccb993a135fccffe932b365182bec7fa16c1a48a126b1869c2'
SOURCE='9d8123c68ff47474a4c16030a2ed4bd4e8874e70f405ef2ca56c2cb7e82e7f35'
PROFILE='ff2872b44d9b02f0af378efb104de406440e6c1d08beea0c7131e5f5f0edae94'
ADMISSION='9460c3e44380a9995375ed64752f7dd3877da39eebdd48075cc3c1d207d6bdcc'
BASELINE_CHANNELS='-all,+timestamp,+pid,+tid,+d2d,+d3d11,+dxgi,err+d3d,warn+d3d,err+winediag,warn+winediag,err+dwrite,warn+dwrite'
CANDIDATE_CHANNELS=BASELINE_CHANNELS+',+dcomp,+loaddll'
ADMISSION_SECONDS=30
CANDIDATE_SECONDS=180
CAPACITY=16*1024*1024
CANDIDATE_ID='proton-11.0-2c-dcomp-c27f058-reference'
CANDIDATE_VERSION='11.0-2c+dcomp-c27f058-reference'
PROTON_SOURCE='5b89db940e0ebe3a137a6009a3589232fe084c09'
WINE_SOURCE='c27f058814b402a5709e073adccd42baa66810b9'
WINE_TREE='622801bdffacb23188c29958c620cc12bf658843'
DLL_POLICY='d2d1,d3d11,dxgi,dcomp=b'
GRAPHICS={f'{arch}/{name}.dll' for arch in ('i386-windows','x86_64-windows')
          for name in ('d2d1','d3d11','dcomp','dxgi','wined3d')}

def read_json(path,maximum=8*1024*1024):
    fd=os.open(path,os.O_RDONLY|os.O_NOFOLLOW)
    with os.fdopen(fd,'rb') as f:
        m=os.fstat(f.fileno())
        if not stat.S_ISREG(m.st_mode) or m.st_uid!=os.getuid() or m.st_size>maximum:
            raise RuntimeError('private JSON refused')
        return json.loads(f.read(maximum+1))

def digest(path,maximum=32*1024*1024):
    fd=os.open(path,os.O_RDONLY|os.O_NOFOLLOW)
    with os.fdopen(fd,'rb') as f:
        m=os.fstat(f.fileno())
        if not stat.S_ISREG(m.st_mode) or m.st_uid!=os.getuid() or m.st_size>maximum:
            raise RuntimeError('admission binary refused')
        return hashlib.file_digest(f,'sha256').hexdigest()

def canonical_digest(value):
    return hashlib.sha256(json.dumps(value,sort_keys=True,separators=(',',':')).encode()).hexdigest()

def tree_identity(root,maximum_entries=14000,maximum_bytes=4*1024*1024*1024):
    root=pathlib.Path(root)
    m=root.lstat()
    if (not root.is_absolute() or root.resolve()!=root or not stat.S_ISDIR(m.st_mode)
            or m.st_uid!=os.getuid() or m.st_mode&0o077):
        raise RuntimeError('candidate root refused')
    rows=[];pending=[root];count=0;total=0
    while pending:
        directory=pending.pop()
        with os.scandir(directory) as entries:items=sorted(entries,key=lambda e:e.name)
        for entry in items:
            p=pathlib.Path(entry.path);rel=p.relative_to(root).as_posix();s=entry.stat(follow_symlinks=False)
            if s.st_uid!=os.getuid():raise RuntimeError('candidate artifact ownership')
            count+=1
            if count>maximum_entries:raise RuntimeError('candidate tree capacity')
            mode=stat.S_IMODE(s.st_mode)
            if stat.S_ISDIR(s.st_mode):
                rows.append(['d',rel,mode]);pending.append(p)
            elif stat.S_ISREG(s.st_mode):
                total+=s.st_size
                if total>maximum_bytes:raise RuntimeError('candidate byte capacity')
                fd=os.open(p,os.O_RDONLY|os.O_NOFOLLOW)
                with os.fdopen(fd,'rb') as f:
                    opened=os.fstat(f.fileno())
                    if (opened.st_dev,opened.st_ino,opened.st_size)!=(s.st_dev,s.st_ino,s.st_size):
                        raise RuntimeError('candidate artifact replaced')
                    sha=hashlib.file_digest(f,'sha256').hexdigest()
                rows.append(['f',rel,mode,s.st_size,sha])
            elif stat.S_ISLNK(s.st_mode):
                target=os.readlink(p)
                try:p.resolve(strict=True).relative_to(root)
                except (FileNotFoundError,ValueError):raise RuntimeError('candidate link escapes')
                rows.append(['l',rel,mode,target])
            else:raise RuntimeError('candidate artifact type')
    h=hashlib.sha256()
    for row in sorted(rows,key=lambda r:r[1]):
        h.update(json.dumps(row,separators=(',',':')).encode()+b'\n')
    return {'schema':1,'entries':count,'regular_bytes':total,'sha256':h.hexdigest()}

def artifact(path):
    path=pathlib.Path(path);return {'path':str(path),'sha256':digest(path,maximum=4*1024*1024*1024)}

def candidate_registration(path,registration):
    manifest=read_json(path)
    keys={'schema','kind','source','build_recipe','base_runner_sha256','root','tree','runner','graphics','dll_overrides'}
    source=manifest.get('source')
    source_keys={'proton_distribution_source_commit','wine_commit','wine_tree','wine_configure','wine_inf_sha256'}
    original=registration['environment']['runner']
    if (not isinstance(manifest,dict) or set(manifest)!=keys or manifest['schema']!=1
            or manifest['kind']!='blackhole_dcomp_reference_runner' or not isinstance(source,dict)
            or set(source)!=source_keys or source['proton_distribution_source_commit']!=PROTON_SOURCE
            or source['wine_commit']!=WINE_SOURCE or source['wine_tree']!=WINE_TREE
            or source['wine_configure']!=['--enable-archs=i386,x86_64']
            or manifest['base_runner_sha256']!=canonical_digest(original)
            or manifest['dll_overrides']!=DLL_POLICY):
        raise RuntimeError('candidate manifest refused')
    root=pathlib.Path(manifest['root'])
    if tree_identity(root)!=manifest['tree']:raise RuntimeError('candidate tree changed')
    wine_inf=artifact(root/'files/share/wine/wine.inf')
    if source['wine_inf_sha256']!=wine_inf['sha256']:raise RuntimeError('candidate wine policy changed')
    recipe=manifest['build_recipe']
    if not isinstance(recipe,dict) or set(recipe)!= {'path','sha256'} or artifact(recipe['path'])!=recipe:
        raise RuntimeError('candidate build recipe changed')
    runner=manifest['runner']
    if (not isinstance(runner,dict) or set(runner)!={'id','version','proton','entry_point','files'}
            or runner['id']!=CANDIDATE_ID or runner['version']!=CANDIDATE_VERSION
            or runner['entry_point']!=original['entry_point']):
        raise RuntimeError('candidate runner refused')
    try:pathlib.Path(runner['proton']).resolve(strict=True).relative_to(root)
    except (FileNotFoundError,ValueError):raise RuntimeError('candidate proton outside root')
    original_files={a['path']:a for a in original['files']}
    if original['entry_point'] not in original_files or original['proton'] not in original_files:
        raise RuntimeError('base runner incomplete')
    graphics=manifest['graphics']
    if not isinstance(graphics,dict) or set(graphics)!=GRAPHICS:raise RuntimeError('candidate graphics set')
    expected=[]
    for name in sorted(GRAPHICS):
        a=graphics[name]
        if (not isinstance(a,dict) or set(a)!= {'path','sha256'}
                or pathlib.Path(a['path'])!=root/'files/lib/wine'/name or artifact(a['path'])!=a):
            raise RuntimeError('candidate graphics changed')
        expected.append(a)
    entry=original_files[original['entry_point']]
    proton=artifact(runner['proton'])
    if proton['sha256']!=original_files[original['proton']]['sha256']:
        raise RuntimeError('candidate proton differs')
    expected.extend([entry,proton])
    if (not isinstance(runner['files'],list) or len(runner['files'])!=len(expected)
            or {json.dumps(a,sort_keys=True) for a in runner['files']}
                != {json.dumps(a,sort_keys=True) for a in expected}):
        raise RuntimeError('candidate runner artifacts differ')
    result=json.loads(json.dumps(registration));result['environment']['runner']=runner
    return result,digest(path),manifest

def graphics_environment(base):
    result=dict(base);prior=result.get('WINEDLLOVERRIDES','')
    names={name.lower() for item in prior.split(';') if item
           for name in item.partition('=')[0].split(',') if name}
    if names.intersection({'d2d1','d3d11','dxgi','dcomp'}):raise RuntimeError('graphics override conflict')
    result['WINEDLLOVERRIDES']=(prior+';' if prior else '')+DLL_POLICY
    result.update(PROTON_LOG='0',PROTON_USE_WINED3D='1',PROTON_DISABLE_NVAPI='1',
        PROTON_DLL_COPY='*',WINEDEBUG=CANDIDATE_CHANNELS,
        DXVK_LOG_LEVEL='none',VKD3D_DEBUG='none')
    return result

def prefix_graphics(registration,candidate):
    prefix=pathlib.Path(registration['environment']['root'])/'compatdata/pfx/drive_c/windows'
    root=pathlib.Path(candidate['root'])
    graphics=candidate['graphics']
    for windows,arch in (('system32','x86_64-windows'),('syswow64','i386-windows')):
        for name in ('d2d1','d3d11','dcomp','dxgi','wined3d'):
            path=prefix/windows/f'{name}.dll';s=path.lstat()
            expected=root/'files/lib/wine'/arch/f'{name}.dll'
            sealed=graphics[f'{arch}/{name}.dll']
            linked=(stat.S_ISLNK(s.st_mode) and path.resolve(strict=True)==expected
                    and artifact(path.resolve(strict=True))==sealed)
            copied=(stat.S_ISREG(s.st_mode) and s.st_nlink==1
                    and digest(path,maximum=4*1024*1024*1024)==sealed['sha256'])
            if s.st_uid!=os.getuid() or not (linked or copied):
                raise RuntimeError('candidate prefix graphics changed')

def service_inactive():
    value=subprocess.check_output(['systemctl','--user','show','linux-vst-bridge.service',
        '--property=ActiveState','--value'],text=True,timeout=5).strip()
    if value!='inactive':raise RuntimeError('bridge service active')

def locked(path,mode):
    fd=os.open(path,os.O_RDWR|os.O_CREAT|os.O_NOFOLLOW,0o600)
    try:
        m=os.fstat(fd)
        if not stat.S_ISREG(m.st_mode) or m.st_uid!=os.getuid() or m.st_nlink!=1 or m.st_mode&0o077:
            raise RuntimeError('manager lock refused')
        fcntl.flock(fd,mode|fcntl.LOCK_NB)
        return fd
    except BaseException:
        os.close(fd);raise

def bounded_entries(path,maximum):
    values=[]
    with os.scandir(path) as entries:
        for entry in entries:
            if len(values)==maximum:raise RuntimeError('owner capacity refused')
            values.append(pathlib.Path(entry.path))
    return values

def validate_admission(value):
    keys={'schema','profile_fingerprint','registration','accessibility_probe_permitted','maximum_seconds','maximum_records'}
    if (not isinstance(value,dict) or set(value)!=keys or value['schema']!=1
            or value['maximum_seconds']!=ADMISSION_SECONDS or value['maximum_records']!=16384
            or value['accessibility_probe_permitted'] is not True):
        raise RuntimeError('managed admission refused')
    reg=value['registration']
    facts=(value['profile_fingerprint'],reg['metadata']['class_id'],reg['environment']['id'],
           reg['module']['sha256'],reg['host']['sha256'],reg['host_source_sha256'])
    if facts!=(PROFILE,CLASS,ENVIRONMENT,MODULE,HOST,SOURCE):
        raise RuntimeError('managed admission changed')
    return reg

def existing_owners(root,forbidden_sessions=None):
    leases=root/'runtime/leases';envs=bounded_entries(root/'environments',128)
    paths=bounded_entries(leases,64) if leases.exists() else []
    rows=[]
    for lease in paths:
        sid=lease.stem
        if lease.suffix!='.json' or len(sid)!=32 or any(c not in '0123456789abcdef' for c in sid):
            raise RuntimeError('owner lease refused')
        report=pathlib.Path(read_json(lease))
        if report.parent!=root/'runtime/results':raise RuntimeError('owner lease refused')
        matches=[p/'compatdata/pfx/drive_c/bridge/sessions'/sid/'owner.json' for p in envs]
        matches=[p for p in matches if p.exists()]
        if len(matches)!=1:raise RuntimeError('owner lease unresolved')
        if forbidden_sessions is not None and matches[0].parent.parent==forbidden_sessions:
            raise RuntimeError('candidate environment owner active')
        owner=read_json(matches[0])
        if owner.get('session')!=sid or owner.get('report')!=str(report) or type(owner.get('keeper')) is not bool:
            raise RuntimeError('owner lease changed')
        if owner['keeper'] and report.name!=f'environment-{sid}.json':raise RuntimeError('owner lease changed')
        rows.append(owner)
    if any(not row['keeper'] for row in rows):raise RuntimeError('non-keeper owner active')
    return rows

class Capture:
    def __init__(self,path,capacity=CAPACITY):
        self.fd=os.open(path,os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW,0o600)
        self.source=None;self.capacity=capacity;self.retained=0;self.discarded=0
    def attach(self,fd):
        if self.source is not None:raise RuntimeError('graphics stderr ambiguous')
        self.source=fd
    def consume(self,fd,data):
        if fd!=self.source:return
        count=min(len(data),self.capacity-self.retained)
        view=memoryview(data)[:count]
        while view:
            n=os.write(self.fd,view);view=view[n:];self.retained+=n
        self.discarded+=len(data)-count
    def close(self):
        os.fsync(self.fd);os.close(self.fd)

def run(admission_binary,candidate_manifest,observation_id):
    if len(observation_id)!=32 or any(c not in '0123456789abcdef' for c in observation_id):
        raise RuntimeError('observation identity refused')
    if digest(admission_binary)!=ADMISSION:raise RuntimeError('admission binary changed')
    root=pathlib.Path.home()/'.local/share/linux-vst-bridge/managed'
    registry_before=digest(root/'registry.json')
    admitted=json.loads(subprocess.check_output(
        [str(admission_binary),'admit-managed-observation',CLASS],timeout=20))
    reg=validate_admission(admitted)
    service=locked(root/'service.lock',fcntl.LOCK_EX);service_inactive()
    registry=locked(root/'registry.lock',fcntl.LOCK_EX)
    if digest(root/'registry.json')!=registry_before:raise RuntimeError('registry changed during admission')
    envroot=pathlib.Path(reg['environment']['root']);sessions=envroot/'compatdata/pfx/drive_c/bridge/sessions'
    owners=existing_owners(root,sessions)
    operation=locked(envroot/'operation.lock',fcntl.LOCK_EX)
    reg,candidate_sha,candidate=candidate_registration(candidate_manifest,reg)
    prefix_graphics(reg,candidate)
    sw=read_json(root/'software.json');supervisor=sw['supervisor']
    if digest(supervisor['path'])!=supervisor['sha256']:raise RuntimeError('installed supervisor changed')
    sys.path.insert(0,str(pathlib.Path(supervisor['path']).parent));import session
    for item in [sw['supervisor'],sw['ownership'],reg['host'],reg['module'],*reg['environment']['runner']['files']]:session.verify(item)
    session.verify({'path':str(pathlib.Path(reg['host']['path']).with_name('host-source-manifest.json')),'sha256':SOURCE})
    base=root/'runtime/graphics-diagnostics'
    if not base.exists():base.mkdir(mode=0o700)
    session.private_directory(base);out=base/observation_id;out.mkdir(mode=0o700)
    results=root/'runtime/results';leases=root/'runtime/leases';session.private_directory(results);session.private_directory(leases)
    sid=os.urandom(16).hex();directory=envroot/'compatdata/pfx/drive_c/bridge/sessions'/sid;directory.mkdir(mode=0o700)
    # Keep the manager's existing vendor-access lease schema so an uncertain
    # cleanup remains an intelligible, blocking owner rather than an orphan.
    report=results/f'windows-{sid}.json';lease=leases/f'{sid}.json'
    spec={'registration':reg,'onboarding_home':True,'session':sid,'directory':str(directory),'report':str(report),
          'lease':str(lease),'inspect':False,'first_audio':False,'keeper':False,'binding_sent':False,
          'vendor_access':True,'shared_inspection':False,'shared_runtime':False,'transport':None}
    session.atomic(directory/'owner.json',spec);session.atomic(lease,str(report))
    session.atomic(out/'active.private.json',{'schema':1,'mode':'unpublished_dcomp_reference',
        'session':sid,'directory':str(directory),'report':str(report),'candidate_manifest_sha256':candidate_sha,
        'candidate_runner_id':candidate['runner']['id'],'baseline_profile_fingerprint':PROFILE,
        'publication_accepted':False})
    trace=Capture(out/'wine.private.log');original_env=session.environment;original_popen=session.subprocess.Popen;original_read=os.read
    expected,_=session.command(spec)
    def environment(value):
        if value!=reg:raise RuntimeError('graphics registration changed')
        return graphics_environment(original_env(value))
    def popen(*args,**kwargs):
        child=original_popen(*args,**kwargs)
        if args and args[0]==expected and child.stderr:trace.attach(child.stderr.fileno())
        return child
    def read(fd,count):
        data=original_read(fd,count);trace.consume(fd,data);return data
    session.environment=environment;session.subprocess.Popen=popen;os.read=read
    prior_alarm=signal.getsignal(signal.SIGALRM);deadline_hit=[False]
    def deadline(_signal,_frame):
        deadline_hit[0]=True
        # session.run owns SIGTERM and converts it to its normal supervised stop.
        os.kill(os.getpid(),signal.SIGTERM)
    signal.signal(signal.SIGALRM,deadline);signal.setitimer(signal.ITIMER_REAL,CANDIDATE_SECONDS)
    result=None;failure=None
    try:result=session.run(spec)
    except BaseException as exc:failure=type(exc).__name__
    finally:
        signal.setitimer(signal.ITIMER_REAL,0);signal.signal(signal.SIGALRM,prior_alarm)
        session.environment=original_env;session.subprocess.Popen=original_popen;os.read=original_read;trace.close()
    failure='candidate_deadline' if deadline_hit[0] else failure or (result and result.get('error'))
    clean=bool(result and result.get('cleanup_confirmed') and result.get('transport_retired'))
    records=[] if result is None else result.get('records',[])
    opened=any(r.get('state')=='ap12_vendor_access_open' for r in records if isinstance(r,dict))
    closed=any(r.get('state')=='ap12_vendor_access_closed' for r in records if isinstance(r,dict))
    completed=any(r.get('state')=='scanner_completed' for r in records if isinstance(r,dict))
    normal_close=closed and completed and not deadline_hit[0]
    summary={'schema':1,'kind':'blackhole_dcomp_reference_graphics_trace','mode':'unpublished_candidate_vendor_access_no_daw_audio',
             'baseline_profile_fingerprint':PROFILE,'session':sid,'maximum_observation_seconds':CANDIDATE_SECONDS,
             'candidate_manifest_sha256':candidate_sha,'candidate_runner_id':candidate['runner']['id'],
             'publication_accepted':False,
             'wine_channels':CANDIDATE_CHANNELS,'retained_bytes':trace.retained,'discarded_bytes':trace.discarded,
             'editor_open_observed':opened,'graceful_editor_close_observed':normal_close,
             'cleanup_confirmed':clean,'failure':failure,'keeper_count':len(owners),'lease_retained':not clean}
    session.atomic(out/'result.private.json',summary)
    if clean:lease.unlink()
    os.close(operation);os.close(registry);os.close(service)
    if failure or not clean or not opened:raise RuntimeError('graphics observation incomplete')
    return {'completed':True,'retained_bytes':trace.retained,'discarded_bytes':trace.discarded,
            'graceful_editor_close_observed':normal_close,'mode':'unpublished_candidate_vendor_access_no_daw_audio'}

def main():
    parser=argparse.ArgumentParser();parser.add_argument('admission_binary',type=pathlib.Path)
    parser.add_argument('candidate_manifest',type=pathlib.Path);parser.add_argument('observation_id');args=parser.parse_args()
    try:value=run(args.admission_binary,args.candidate_manifest,args.observation_id)
    except BaseException:
        print('Graphics observation refused; inspect private retained state.',file=sys.stderr);return 1
    print(json.dumps(value,separators=(',',':')));return 0

if __name__=='__main__':raise SystemExit(main())

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
import threading

CLASS='5653544248496D626C61636B686F6C65'
ENVIRONMENT='8b064373d533b72230cb4009952eaab4'
MODULE='b8a33c57b04eded38d0ca717bb5e3721330439f4931c401ea70e5246b516e90b'
HOST='5fa0907b045df5ccb993a135fccffe932b365182bec7fa16c1a48a126b1869c2'
SOURCE='9d8123c68ff47474a4c16030a2ed4bd4e8874e70f405ef2ca56c2cb7e82e7f35'
PROFILE='ff2872b44d9b02f0af378efb104de406440e6c1d08beea0c7131e5f5f0edae94'
ADMISSION='9460c3e44380a9995375ed64752f7dd3877da39eebdd48075cc3c1d207d6bdcc'
CHANNELS='-all,+timestamp,+pid,+tid,+d2d,+d3d11,+dxgi,err+d3d,warn+d3d,err+winediag,warn+winediag,err+dwrite,warn+dwrite'
SECONDS=30
CAPACITY=16*1024*1024

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
            or value['maximum_seconds']!=SECONDS or value['maximum_records']!=16384
            or value['accessibility_probe_permitted'] is not True):
        raise RuntimeError('managed admission refused')
    reg=value['registration']
    facts=(value['profile_fingerprint'],reg['metadata']['class_id'],reg['environment']['id'],
           reg['module']['sha256'],reg['host']['sha256'],reg['host_source_sha256'])
    if facts!=(PROFILE,CLASS,ENVIRONMENT,MODULE,HOST,SOURCE):
        raise RuntimeError('managed admission changed')
    return reg

def existing_owners(root):
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

def run(admission_binary,observation_id):
    if len(observation_id)!=32 or any(c not in '0123456789abcdef' for c in observation_id):
        raise RuntimeError('observation identity refused')
    if digest(admission_binary)!=ADMISSION:raise RuntimeError('admission binary changed')
    root=pathlib.Path.home()/'.local/share/linux-vst-bridge/managed'
    registry_before=digest(root/'registry.json')
    admitted=json.loads(subprocess.check_output(
        [str(admission_binary),'admit-managed-observation',CLASS],timeout=20))
    reg=validate_admission(admitted)
    registry=locked(root/'registry.lock',fcntl.LOCK_EX)
    if digest(root/'registry.json')!=registry_before:raise RuntimeError('registry changed during admission')
    owners=existing_owners(root)
    envroot=pathlib.Path(reg['environment']['root']);operation=locked(envroot/'operation.lock',fcntl.LOCK_SH)
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
    trace=Capture(out/'wine.private.log');original_env=session.environment;original_popen=session.subprocess.Popen;original_read=os.read
    expected,_=session.command(spec)
    def environment(value):
        if value!=reg:raise RuntimeError('graphics registration changed')
        value=original_env(value);value.update(PROTON_LOG='0',WINEDEBUG=CHANNELS,DXVK_LOG_LEVEL='none',VKD3D_DEBUG='none');return value
    def popen(*args,**kwargs):
        child=original_popen(*args,**kwargs)
        if args and args[0]==expected and child.stderr:trace.attach(child.stderr.fileno())
        return child
    def read(fd,count):
        data=original_read(fd,count);trace.consume(fd,data);return data
    session.environment=environment;session.subprocess.Popen=popen;os.read=read
    expired=threading.Event();timer=threading.Thread(target=lambda:expired.wait(SECONDS) or os.kill(os.getpid(),signal.SIGTERM),daemon=True);timer.start()
    result=None;failure=None
    try:result=session.run(spec)
    except BaseException as exc:failure=type(exc).__name__
    finally:
        expired.set();session.environment=original_env;session.subprocess.Popen=original_popen;os.read=original_read;trace.close()
    failure=failure or (result and result.get('error'))
    clean=bool(result and result.get('cleanup_confirmed') and result.get('transport_retired'))
    records=[] if result is None else result.get('records',[])
    opened=any(r.get('state')=='ap12_vendor_access_open' for r in records if isinstance(r,dict))
    closed=any(r.get('state')=='ap12_vendor_access_closed' for r in records if isinstance(r,dict))
    completed=any(r.get('state')=='scanner_completed' for r in records if isinstance(r,dict))
    summary={'schema':1,'kind':'blackhole_vendor_access_graphics_trace','mode':'vendor_access_no_daw_audio',
             'profile_fingerprint':PROFILE,'session':sid,'maximum_observation_seconds':SECONDS,
             'wine_channels':CHANNELS,'retained_bytes':trace.retained,'discarded_bytes':trace.discarded,
             'editor_open_observed':opened,'graceful_editor_close_observed':closed and completed,
             'cleanup_confirmed':clean,'failure':failure,'keeper_count':len(owners),'lease_retained':not clean}
    session.atomic(out/'result.private.json',summary)
    if clean:lease.unlink()
    os.close(operation);os.close(registry)
    if failure or not clean or not opened:raise RuntimeError('graphics observation incomplete')
    return {'completed':True,'retained_bytes':trace.retained,'discarded_bytes':trace.discarded,
            'graceful_editor_close_observed':closed and completed,'mode':'vendor_access_no_daw_audio'}

def main():
    parser=argparse.ArgumentParser();parser.add_argument('admission_binary',type=pathlib.Path);parser.add_argument('observation_id');args=parser.parse_args()
    try:value=run(args.admission_binary,args.observation_id)
    except BaseException:
        print('Graphics observation refused; inspect private retained state.',file=sys.stderr);return 1
    print(json.dumps(value,separators=(',',':')));return 0

if __name__=='__main__':raise SystemExit(main())

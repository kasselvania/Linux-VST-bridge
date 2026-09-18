"""Closed, committed source-owned package; no vendor bytes or installed mutation."""
import hashlib,json,os,pathlib,shutil,stat,subprocess,tempfile
SOURCES={n:'tools/naui2/'+n for n in ('package.py','campaign.py','supervise.py','readback.py','application.cpp')}
SOURCES.update({n:'bridge-manager/runtime/'+n for n in ('session.py','ownership.py')})
SOURCES.update({'linker.py':'tools/is4/linker.py','Cargo.lock':'bridge-manager/Cargo.lock','Cargo.toml':'bridge-manager/Cargo.toml'})
SOURCES['nad1_stop.h']='tools/is2/nad1_stop.h'
SOURCES['nad1_service.h']='tools/is2/nad1_service.h'
SOURCES.update({'launch.cpp':'tools/is2/launch.cpp','binding.rs':'bridge-manager/examples/naui2_binding.rs','application.rs':'bridge-manager/src/renderer_application.rs','lifecycle.rs':'bridge-manager/src/renderer_session.rs','operator_cli.rs':'bridge-manager/src/operator_cli.rs'})
SOURCES['native_access_callback.h']='tools/is2/native_access_callback.h'
REQUIRED=set(SOURCES)|{'payload.exe','adapter.exe','binding-owner','context.private.json'}
def digest(p):
    with pathlib.Path(p).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def canonical(v):return json.dumps(v,sort_keys=True,separators=(',',':')).encode()+b'\n'
def unique(pairs):
    r={}
    for k,v in pairs:
        if k in r:raise ValueError('duplicate_key')
        r[k]=v
    return r
def read(p):
    if pathlib.Path(p).stat().st_size>8*1024*1024:raise ValueError('record_bound')
    return json.loads(pathlib.Path(p).read_bytes(),object_pairs_hook=unique)
def publish(path,value):
    path=pathlib.Path(path);fd,tmp=tempfile.mkstemp(dir=path.parent)
    try:
        with os.fdopen(fd,'wb') as f:f.write(canonical(value));f.flush();os.fsync(f.fileno())
        os.chmod(tmp,0o400);os.link(tmp,path)
        fd=os.open(path.parent,os.O_RDONLY|os.O_DIRECTORY)
        try:os.fsync(fd)
        finally:os.close(fd)
    finally:os.unlink(tmp)
def verify(package,seal):
    p=pathlib.Path(package)
    if digest(p/'manifest.json')!=seal:raise ValueError('package_seal')
    m=read(p/'manifest.json')
    if set(m)!={'schema','source','files','recipe','payload_size'} or m['schema']!=1 or set(m['files'])!=REQUIRED or set(x.name for x in p.iterdir())!=REQUIRED|{'manifest.json'}:raise ValueError('package_schema')
    if (p/'manifest.json').read_bytes()!=canonical(m):raise ValueError('canonical_manifest')
    for n,h in m['files'].items():
        f=p/n;s=f.lstat()
        if not stat.S_ISREG(s.st_mode) or s.st_uid!=os.getuid() or s.st_nlink!=1 or s.st_mode&0o222 or digest(f)!=h:raise ValueError('package_input_changed')
    if (p/'payload.exe').stat().st_size!=m['payload_size'] or m['payload_size']!=67*1024*1024:raise ValueError('large_fixture_extent')
    return m

def build(repo,out,context):
    repo=pathlib.Path(repo).resolve();out=pathlib.Path(out).resolve()
    def git(*a):return subprocess.check_output(['git','-C',str(repo),*a]).decode().strip()
    if git('status','--porcelain','--untracked-files=no'):raise ValueError('commit_source_before_build')
    source={'head':git('rev-parse','HEAD'),'tree':git('rev-parse','HEAD^{tree}')};out.mkdir(mode=0o700)
    for name,path in SOURCES.items():(out/name).write_bytes(subprocess.check_output(['git','-C',str(repo),'show',source['head']+':'+path]))
    zig=shutil.which('zig')
    with tempfile.TemporaryDirectory(prefix='naui2-build-') as d:
        for name,src,libs in [('payload.exe','application.cpp',[]),('adapter.exe','launch.cpp',['-lbcrypt','-ladvapi32'])]:
            built=pathlib.Path(d)/name
            subprocess.run([zig,'c++','-target','x86_64-windows-gnu','-std=c++20','-O2','-municode','-DUNICODE','-D_UNICODE',str(out/src),*libs,'-o',str(built)],check=True,timeout=180)
            shutil.copyfile(built,out/name)
    with (out/'payload.exe').open('ab') as f:f.truncate(67*1024*1024)
    env=dict(os.environ,IS4_ZIG=zig,CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=str(repo/'tools/is4/linker.py'))
    subprocess.run(['cargo','build','--manifest-path',str(repo/'bridge-manager/Cargo.toml'),'--locked','--release','--target','x86_64-unknown-linux-gnu','--example','naui2_binding'],env=env,check=True,timeout=240)
    shutil.copyfile(repo/'bridge-manager/target/x86_64-unknown-linux-gnu/release/examples/naui2_binding',out/'binding-owner')
    shutil.copyfile(context,out/'context.private.json')
    m={'schema':1,'source':source,'files':{p.name:digest(p) for p in out.iterdir()},'payload_size':67*1024*1024,
       'recipe':{'name':'naui2-x64-cpp20-padded67MiB-rust-binding-v1','zig_sha256':digest(zig),'zig_version':subprocess.check_output([zig,'version']).decode().strip()}}
    for p in out.iterdir():p.chmod(0o500 if p.name=='binding-owner' else 0o400)
    publish(out/'manifest.json',m);seal=digest(out/'manifest.json');verify(out,seal);return seal
if __name__=='__main__':
    import sys
    print(build(*sys.argv[1:]))

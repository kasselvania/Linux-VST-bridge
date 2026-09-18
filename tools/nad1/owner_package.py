"""Exact closed source package for the generated SCM owner qualification."""
import hashlib,json,os,pathlib,shutil,stat,subprocess,tempfile
SOURCES={n:'tools/nad1/'+n for n in ('owner_package.py','owner_campaign.py','owner_supervise.py','service_fixture.cpp','application_fixture.cpp','application_supervise.py','application_campaign.py','lifetime_campaign.py','recovery_campaign.py','child_memory.h','child_memory_campaign.py','callback_campaign.py')}
SOURCES.update({n:'bridge-manager/runtime/'+n for n in ('session.py','ownership.py')})
SOURCES.update({'launch.cpp':'tools/is2/launch.cpp','nad1_service.h':'tools/is2/nad1_service.h','readback.py':'tools/naui2/readback.py','binding.rs':'bridge-manager/examples/nad1_binding.rs','dependency.rs':'bridge-manager/src/native_access_dependency.rs','lifecycle.rs':'bridge-manager/src/dependency_session.rs','operator_cli.rs':'bridge-manager/src/operator_cli.rs','linker.py':'tools/is4/linker.py','Cargo.toml':'bridge-manager/Cargo.toml','Cargo.lock':'bridge-manager/Cargo.lock'})
SOURCES['native_access_recovery.json']='bridge-manager/src/native_access_recovery.json'
SOURCES['renderer_lifecycle.rs']='bridge-manager/src/renderer_session.rs'
SOURCES['nad1_stop.h']='tools/is2/nad1_stop.h'
SOURCES['native_access_callback.h']='tools/is2/native_access_callback.h'
REQUIRED=set(SOURCES)|{'Setup.exe','application.exe','adapter.exe','binding-owner'}
def digest(p):
 with open(p,'rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def canonical(v):return json.dumps(v,sort_keys=True,separators=(',',':')).encode()+b'\n'
def publish(p,v):
 with open(p,'xb') as f:f.write(canonical(v));f.flush();os.fsync(f.fileno());os.fchmod(f.fileno(),0o400)
def read(p):return json.loads(pathlib.Path(p).read_bytes())
def verify(p,seal):
 p=pathlib.Path(p)
 s=(p/'seal.json').lstat()
 if not stat.S_ISREG(s.st_mode) or s.st_nlink!=1 or s.st_uid!=os.getuid() or s.st_mode&0o222 or s.st_size>65536:raise ValueError('seal_file')
 if digest(p/'seal.json')!=seal:raise ValueError('seal_identity')
 m=read(p/'seal.json')
 if set(m)!={'schema','source','files'} or m['schema']!=1 or set(m['files'])!=REQUIRED or {f.name for f in p.iterdir()}!=REQUIRED|{'seal.json'} or (p/'seal.json').read_bytes()!=canonical(m):raise ValueError('seal_shape')
 for name,h in m['files'].items():
  f=p/name;s=f.lstat()
  if not stat.S_ISREG(s.st_mode) or s.st_uid!=os.getuid() or s.st_nlink!=1 or s.st_mode&0o222 or digest(f)!=h:raise ValueError('seal_input')
 return m
def build(repo,out):
 repo=pathlib.Path(repo).resolve();out=pathlib.Path(out);out.mkdir(mode=0o700)
 def git(*a):return subprocess.check_output(['git','-C',str(repo),*a])
 if git('status','--porcelain','--untracked-files=normal'):raise ValueError('source_uncommitted')
 source={'head':git('rev-parse','HEAD').decode().strip(),'tree':git('rev-parse','HEAD^{tree}').decode().strip()}
 for n,p in SOURCES.items():(out/n).write_bytes(git('show',source['head']+':'+p))
 with tempfile.TemporaryDirectory() as d:
  for name,src,flags in [('application.exe','application_fixture.cpp',['-ladvapi32']),('Setup.exe','service_fixture.cpp',['-lws2_32','-ladvapi32']),('adapter.exe','launch.cpp',['-DNAD1_GENERATED_SERVICE','-lbcrypt','-ladvapi32'])]:
   output=pathlib.Path(d)/name;subprocess.run(['zig','c++','-target','x86_64-windows-gnu','-std=c++20','-O2','-municode',str(out/src),*flags,'-o',str(output)],check=True,timeout=180);shutil.copyfile(output,out/name)
 env=dict(os.environ,IS4_ZIG=shutil.which('zig'),CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=str(repo/'tools/is4/linker.py'))
 subprocess.run(['cargo','+1.95.0','build','--manifest-path',str(repo/'bridge-manager/Cargo.toml'),'--locked','--release','--target','x86_64-unknown-linux-gnu','--example','nad1_binding'],env=env,check=True,timeout=300)
 shutil.copyfile(repo/'bridge-manager/target/x86_64-unknown-linux-gnu/release/examples/nad1_binding',out/'binding-owner')
 for f in out.iterdir():f.chmod(0o500 if f.name=='binding-owner' else 0o400)
 m={'schema':1,'source':source,'files':{f.name:digest(f) for f in out.iterdir()}}
 publish(out/'seal.json',m);return digest(out/'seal.json')
if __name__=='__main__':
 import sys
 print(build(*sys.argv[1:]))

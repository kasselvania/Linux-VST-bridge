"""Sealed source-owned lead qualification, independent of production admission."""
import hashlib,json,os,pathlib,shutil,stat,subprocess,tempfile
SOURCES={n:'tools/nad1/'+n for n in ('lead_package.py','lead_run.py','lead.cpp')}
SOURCES.update({n:'bridge-manager/runtime/'+n for n in ('session.py','ownership.py','dependency_process.py')})
def digest(p):
 with open(p,'rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def canonical(v):return json.dumps(v,sort_keys=True,separators=(',',':')).encode()+b'\n'
def publish(p,v):
 with open(p,'xb') as f:f.write(canonical(v));f.flush();os.fsync(f.fileno());os.fchmod(f.fileno(),0o400)
def verify(p,seal):
 p=pathlib.Path(p)
 if digest(p/'seal.json')!=seal:raise ValueError('seal_identity')
 m=json.loads((p/'seal.json').read_bytes())
 if set(m)!= {'schema','source','files'} or m['schema']!=1 or set(m['files'])!=set(SOURCES)|{'NTKDaemon.exe'} or {f.name for f in p.iterdir()}!=set(m['files'])|{'seal.json'}:raise ValueError('seal_shape')
 if (p/'seal.json').read_bytes()!=canonical(m):raise ValueError('seal_canonical')
 for name,h in m['files'].items():
  s=(p/name).lstat()
  if not stat.S_ISREG(s.st_mode) or s.st_nlink!=1 or s.st_uid!=os.getuid() or s.st_mode&0o222 or digest(p/name)!=h:raise ValueError('seal_input')
 return m
def build(repo,out):
 repo=pathlib.Path(repo).resolve();out=pathlib.Path(out);out.mkdir(mode=0o700)
 def git(*a):return subprocess.check_output(['git','-C',str(repo),*a])
 if git('status','--porcelain','--untracked-files=no'):raise ValueError('source_uncommitted')
 source={'head':git('rev-parse','HEAD').decode().strip(),'tree':git('rev-parse','HEAD^{tree}').decode().strip()}
 for n,p in SOURCES.items():(out/n).write_bytes(git('show',source['head']+':'+p))
 with tempfile.TemporaryDirectory() as d:
  exe=pathlib.Path(d)/'NTKDaemon.exe'
  subprocess.run(['zig','c++','-target','x86_64-windows-gnu','-std=c++20','-O2','-municode',str(out/'lead.cpp'),'-o',str(exe)],check=True,timeout=180)
  shutil.copyfile(exe,out/'NTKDaemon.exe')
 for f in out.iterdir():f.chmod(0o400)
 m={'schema':1,'source':source,'files':{f.name:digest(f) for f in out.iterdir()}}
 publish(out/'seal.json',m);return digest(out/'seal.json')
if __name__=='__main__':
 import sys
 print(build(*sys.argv[1:]))

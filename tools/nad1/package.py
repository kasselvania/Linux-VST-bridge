"""Detached seal over an exact, closed committed diagnostic package."""
import os,pathlib,subprocess,sys
# Local tests import shared source; sealed packages carry the same committed files.
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parents[1]/'naui1'))
sys.path.insert(1,str(pathlib.Path(__file__).resolve().parents[2]/'bridge-manager/runtime'))
from common import read,decode,digest,canonical,require,publish
SOURCES={n:'tools/nad1/'+n for n in ['package.py','observer.py','census.py','pe_metadata.py','classify.py','input.json']}
SOURCES['dependency_process.py']='bridge-manager/runtime/dependency_process.py'
SOURCES['ownership.py']='bridge-manager/runtime/ownership.py'
SOURCES.update({n:'tools/naui1/'+n for n in ['common.py','identity.py']})
def verify(root):
    root=pathlib.Path(root);require({p.name for p in root.iterdir()}==set(SOURCES)|{'seal.json'},'seal_file_set')
    raw=read(root/'seal.json',65536);m=decode(raw)
    require(set(m)=={'schema','source_head','source_tree','files'} and type(m['schema']) is int and m['schema']==1 and set(m['files'])==set(SOURCES),'seal_schema')
    require(canonical(m)+b'\n'==raw,'seal_canonical')
    for name,h in m['files'].items():
        require(not (root/name).stat().st_mode&0o222,'seal_writable');read(root/name,256*1024,expected=h)
    return m,digest(raw)
def build(repo,out):
    repo=pathlib.Path(repo);out=pathlib.Path(out)
    def git(*a):return subprocess.check_output(['git','-C',str(repo),*a])
    require(not git('status','--porcelain','--','tools/nad1','tools/naui1'),'source_uncommitted')
    head=git('rev-parse','HEAD').decode().strip();tree=git('rev-parse','HEAD^{tree}').decode().strip();out.mkdir(mode=0o700)
    files={}
    for name,source in SOURCES.items():
        raw=git('show',head+':'+source);files[name]=digest(raw)
        with (out/name).open('xb') as f:f.write(raw);f.flush();os.fsync(f.fileno());os.fchmod(f.fileno(),0o400)
    publish(out/'seal.json',{'schema':1,'source_head':head,'source_tree':tree,'files':files});return verify(out)
if __name__=='__main__':
    require(len(sys.argv)==3,'package_usage');print(canonical(build(*sys.argv[1:])[0]).decode())

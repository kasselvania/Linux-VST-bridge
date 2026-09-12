"""Private development helper custody. No product installation or arbitrary EXE.

The Rust admission command verifies ordinary publication. This edge resolves an
active lease, verifies the installed runtime helpers, and launches only the four
build-pinned UIO1 files. Run under a dedicated transient user unit: its cgroup
owns helpers, never the already-running vendor host. All output stays private.
"""
import hashlib
import importlib.util
import json
import os
import pathlib
import selectors
import stat
import subprocess
import sys
import time

HERE=pathlib.Path(__file__).resolve().parent
FILES={'uio1-observer.exe','uio1-hook.dll','uio1-accessibility.exe','uio1-tests.exe'}

def digest(path):
    with open(path,'rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()

def sealed_bytes(path,expected):
    # Diagnostic helpers are small. Refuse replacement, symlink and oversized
    # inputs instead of reading unbounded bytes after an earlier hash check.
    fd=os.open(path,os.O_RDONLY|os.O_NOFOLLOW)
    with os.fdopen(fd,'rb') as f:
        s=os.fstat(f.fileno())
        if not stat.S_ISREG(s.st_mode) or s.st_uid!=os.getuid() or s.st_size>4*1024*1024:
            raise RuntimeError('diagnostic artifact ownership/size')
        data=f.read(4*1024*1024+1)
    if len(data)!=s.st_size or hashlib.sha256(data).hexdigest()!=expected:raise RuntimeError('diagnostic artifact replaced')
    return data

def private_json(path,value):
    with os.fdopen(os.open(path,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600),'w') as f:
        json.dump(value,f,indent=2);f.write('\n')

def verified_package(source):
    manifest=json.loads((HERE/'package.json').read_text())
    if set(manifest['files'])!=FILES:raise RuntimeError('closed diagnostic file set')
    for name,sha in manifest['files'].items():
        p=source/name
        sealed_bytes(p,sha)
    return manifest

class Context:
    def __init__(self,admission_binary,class_id,package,uir1=False,if1=False):
        os.umask(0o077)
        if uir1 and if1:raise RuntimeError('conflicting exact diagnostic purpose')
        command=['admit-if1'] if if1 else (['admit-uir1'] if uir1 else ['admit',class_id])
        self.admission=json.loads(subprocess.check_output([str(admission_binary),*command],timeout=20))
        if self.admission['registration']['metadata']['class_id']!=class_id:raise RuntimeError('admission class mismatch')
        if self.admission['schema']!=1:raise RuntimeError('admission schema')
        self.reg=self.admission['registration']
        self.root=pathlib.Path.home()/'.local/share/linux-vst-bridge/managed'
        self.software=json.loads((self.root/'software.json').read_text())
        for k in ('supervisor','ownership'):
            a=self.software[k]
            if digest(a['path'])!=a['sha256']:raise RuntimeError('installed helper differs')
        supervisor=pathlib.Path(self.software['supervisor']['path'])
        sys.path.insert(0,str(supervisor.parent))
        spec=importlib.util.spec_from_file_location('uio1_installed_runtime',supervisor)
        self.runtime=importlib.util.module_from_spec(spec);spec.loader.exec_module(self.runtime)
        self.env=self.runtime.environment(self.reg)
        self.prefix=pathlib.Path(self.reg['environment']['root'])/'compatdata/pfx'
        self.manifest=verified_package(package)
        self.package=self.prefix/'drive_c/bridge/diagnostics'/('uio1-'+self.manifest['source_head'])
        if self.package.resolve()!=self.package:raise RuntimeError('diagnostic directory alias')
        if self.package.exists():verified_package(self.package)
        else:
            self.package.mkdir(parents=True,mode=0o700)
            for name,sha in self.manifest['files'].items():
                # Copy once into an owned immutable directory, then check bytes.
                data=sealed_bytes(package/name,sha)
                with os.fdopen(os.open(self.package/name,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o500),'wb') as f:f.write(data)
            verified_package(self.package)
        self.session=self.resolve(class_id)
        self.fault=self.runtime.FaultStatus(pathlib.Path(self.session['directory']),self.session['session'])

    def resolve(self,class_id):
        sessions=[]
        leases=list((self.root/'runtime/leases').glob('*.json'))
        if len(leases)>64:raise RuntimeError('lease bound')
        for lease in leases:
            sid=lease.stem
            if len(sid)!=32 or any(c not in '0123456789abcdef' for c in sid):raise RuntimeError('lease identity')
            p=self.prefix/'drive_c/bridge/sessions'/sid/'owner.json'
            if not p.exists():continue
            s=json.loads(p.read_text());r=s['registration']
            if s.get('keeper') or r['metadata']['class_id']!=class_id:continue
            if s['session']!=sid or json.loads(lease.read_text())!=s['report']:raise RuntimeError('lease/session changed')
            # Runtime registration deliberately omits native/discovery strings.
            # Compare the actual authority fields, never friendly-name metadata.
            if any(r[k]!=self.reg[k] for k in ('module','host','host_source_sha256','environment','compatibility')):
                raise RuntimeError('active session artifacts differ')
            directory,_=self.runtime.session_directories(s)
            if not (directory/'ap11.ui').is_file():raise RuntimeError('editor mapping absent')
            sessions.append(s)
        if len(sessions)!=1:raise RuntimeError('one exact active editor session required')
        return sessions[0]

    def helper(self,name,args,output,maximum_seconds):
        if name not in FILES or name.endswith('.dll'):raise RuntimeError('unknown diagnostic helper')
        if name=='uio1-accessibility.exe' and not self.admission['accessibility_probe_permitted']:
            raise RuntimeError('profile prohibits accessibility')
        self.runtime.verify({'path':str(self.package/name),'sha256':self.manifest['files'][name]})
        runner=self.reg['environment']['runner']
        command=[runner['entry_point'],'--verb=run','--',runner['proton'],'runinprefix',self.runtime.windows(self.package/name,self.prefix),*map(str,args)]
        return Helper(self.runtime,command,self.env,self.package,output,maximum_seconds)

    def census(self,output):
        snap=self.fault.snapshot();owner=snap.get('owner') or {}
        if not snap.get('editor',{}).get('open') or owner.get('stage')!=25:raise RuntimeError('live editor owner absent')
        h=self.helper('uio1-observer.exe',['census',owner['process_id']],output,10)
        result=h.finish()
        if result['exit']!=0 or result['overflow']:raise RuntimeError('census incomplete')
        rows=[json.loads(line) for line in output.read_text().splitlines() if line.startswith('{')]
        process=next(r for r in rows if r['type']=='process')
        return process,rows,snap

class Helper:
    CAPACITY=524288
    def __init__(self,runtime,command,env,cwd,output,seconds):
        if not 1<=seconds<=190:raise ValueError('helper lifetime')
        self.runtime=runtime;self.output=output;self.deadline=time.monotonic()+seconds
        self.file=os.fdopen(os.open(output,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600),'wb')
        self.process=subprocess.Popen(command,env=env,cwd=cwd,stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True)
        self.tracker=runtime.ProcessTracker(self.process.pid);self.selector=selectors.DefaultSelector()
        for pipe,tag in [(self.process.stdout,1),(self.process.stderr,2)]:
            os.set_blocking(pipe.fileno(),False);self.selector.register(pipe,selectors.EVENT_READ,tag)
        self.kept=0;self.overflow=0;self.stderr=0;self.closed=False
    def poll(self):
        self.tracker.update()
        for key,_ in self.selector.select(0):
            b=os.read(key.fileobj.fileno(),8192)
            if not b:self.selector.unregister(key.fileobj);continue
            if key.data==2:self.stderr+=len(b)
            take=min(len(b),self.CAPACITY-self.kept);self.file.write(b[:take]);self.kept+=take;self.overflow+=len(b)-take
        if time.monotonic()>self.deadline and (self.process.poll() is None or self.selector.get_map()):raise TimeoutError('diagnostic helper deadline')
        return self.process.poll()
    def finish(self):
        if self.closed:raise RuntimeError('helper already retired')
        error=None
        try:
            while self.process.poll() is None:self.poll();time.sleep(.02)
            while self.selector.get_map():self.poll();time.sleep(.01)
        except BaseException as e:error=e
        finally:
            self.tracker.update()
            cleanup=self.runtime.cleanup_process(self.process,list(self.tracker.owned))
            self.selector.close();self.process.stdout.close();self.process.stderr.close();self.file.close();self.closed=True
        result=dict(exit=self.process.returncode,kept=self.kept,overflow=self.overflow,stderr_bytes=self.stderr,cleanup=cleanup)
        private_json(self.output.with_suffix('.result.json'),result)
        if error:raise error
        return result

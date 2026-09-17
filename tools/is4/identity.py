"""Closed, canonical IS4 package and experiment identity. Paths stay private."""
import hashlib,json,os,pathlib,re,stat,struct,tempfile

SOURCES={name:'tools/is4/'+name for name in ('identity.py','package.py','campaign.py','supervise.py','run.py','comparison.py')}
SOURCES.update({name:'bridge-manager/runtime/'+name for name in ('session.py','ownership.py')})
SOURCES.update({name:'tools/is3/'+name for name in ('report.py','environment.py','capability.cpp')})
SOURCES.update({'policy.rs':'bridge-manager/src/installer_policy.rs','policy-example.rs':'bridge-manager/examples/is4_policy.rs','linker.py':'tools/is4/linker.py','policy-owner':None})
REQUIRED=frozenset(SOURCES)|{'payload.exe'}
MANIFEST='fixture-manifest.json'
OWNERS=('manager','operator_frontend','supervisor','ownership','installer_launch')

def canonical(v):return json.dumps(v,sort_keys=True,separators=(',',':'),ensure_ascii=True).encode()+b'\n'
def hashed(v):return hashlib.sha256(canonical(v)).hexdigest()
def digest(p):
    with pathlib.Path(p).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def exact(v,keys):
    if not isinstance(v,dict) or set(v)!=set(keys):raise ValueError('closed_schema')
def sha(v):
    if not isinstance(v,str) or not re.fullmatch('[a-f0-9]{64}',v):raise ValueError('digest')
def no_duplicates(pairs):
    result={}
    for k,v in pairs:
        if k in result:raise ValueError('duplicate_json_key')
        result[k]=v
    return result

def read_bytes(p):
    p=pathlib.Path(p)
    with os.fdopen(os.open(p,os.O_RDONLY|os.O_NOFOLLOW),'rb') as f:
        before=os.fstat(f.fileno())
        if not stat.S_ISREG(before.st_mode) or before.st_uid!=os.getuid() or before.st_size>2*1024**2:raise ValueError('private_record_type_bound')
        data=f.read(2*1024**2+1);after=os.fstat(f.fileno())
    if len(data)!=before.st_size or (before.st_size,before.st_mtime_ns,before.st_ctime_ns)!=(after.st_size,after.st_mtime_ns,after.st_ctime_ns):raise ValueError('record_changed')
    return data

def read_json(p):return json.loads(read_bytes(p),object_pairs_hook=no_duplicates)

def atomic_new(path,value):
    """Publish complete bytes without replacing retained evidence; fsync parent."""
    path=pathlib.Path(path);data=canonical(value)
    fd,tmp=tempfile.mkstemp(prefix='.is4-',dir=path.parent)
    try:
        with os.fdopen(fd,'wb') as f:f.write(data);f.flush();os.fsync(f.fileno())
        os.chmod(tmp,0o400);os.link(tmp,path,follow_symlinks=False)
        d=os.open(path.parent,os.O_RDONLY|os.O_DIRECTORY)
        try:os.fsync(d)
        finally:os.close(d)
    finally:os.unlink(tmp)

def architecture(path):
    with pathlib.Path(path).open('rb') as f:
        header=f.read(64)
        if len(header)!=64 or header[:2]!=b'MZ':raise ValueError('payload_pe')
        offset=struct.unpack_from('<I',header,60)[0]
        if offset>1024**2:raise ValueError('payload_pe_bound')
        f.seek(offset);pe=f.read(6)
    if pe[:4]!=b'PE\0\0':raise ValueError('payload_pe')
    machine=struct.unpack_from('<H',pe,4)[0]
    if machine not in (0x14c,0x8664):raise ValueError('payload_architecture')
    return 'x86' if machine==0x14c else 'x64'

def artifact(a):
    # Public exact location digest, never operator/home paths.
    p=pathlib.Path(a['path']);s=p.lstat()
    if not stat.S_ISREG(s.st_mode) or digest(p)!=a['sha256']:raise ValueError('artifact_drift')
    return {'location_sha256':hashlib.sha256(str(p).encode()).hexdigest(),'sha256':a['sha256'],'size':s.st_size}

def runner_identity(r):
    files=sorted([artifact(a) for a in r['files']],key=lambda a:a['location_sha256'])
    if not files or len({a['location_sha256'] for a in files})!=len(files):raise ValueError('runner_files')
    def declared(name):
        location=hashlib.sha256(r[name].encode()).hexdigest()
        matches=[f for f in files if f['location_sha256']==location]
        if len(matches)!=1:raise ValueError('runner_executable_not_in_verified_set')
        return matches[0]
    return {'id':r['id'],'record_sha256':hashed(r),'entry_point':declared('entry_point'),'proton':declared('proton'),'files':files}

def select_runner(registry,expected):
    candidates={}
    for row in registry['classes'].values():
        r=row['registration']['environment']['runner']
        if r['id']==expected['id']:candidates[hashed(r)]=r
    if len(candidates)!=1:raise ValueError('runner_absent_or_ambiguous')
    selected=next(iter(candidates.values()))
    if canonical(runner_identity(selected))!=canonical(expected):raise ValueError('runner_generation_drift')
    return selected

def installed_identity(software):
    return {'record_sha256':hashed(software),'artifacts':{k:artifact(software[k]) for k in OWNERS}}

def validate_identity(v):
    exact(v,('schema','payload','runner','powershell_images','installed','sources','manifest_sha256','source','baseline_windows_environment'))
    if v['schema']!=2:raise ValueError('identity_schema')
    exact(v['payload'],('sha256','size','architecture'));sha(v['payload']['sha256'])
    if type(v['payload']['size'])!=int or v['payload']['size']<=0 or v['payload']['architecture'] not in ('x86','x64'):raise ValueError('payload_identity')
    exact(v['runner'],('id','record_sha256','entry_point','proton','files'));sha(v['runner']['record_sha256'])
    if not v['runner']['files']:raise ValueError('runner_files')
    def art(a):
        exact(a,('sha256','size','location_sha256'));sha(a['sha256']);sha(a['location_sha256'])
        if type(a['size'])!=int or a['size']<0:raise ValueError('artifact_size')
    for a in v['runner']['files']:art(a)
    for k in ('entry_point','proton'):
        art(v['runner'][k])
        if v['runner'][k]['size']==0:raise ValueError('empty_runner_executable')
        if v['runner'][k] not in v['runner']['files']:raise ValueError('runner_set')
    if len({a['location_sha256'] for a in v['runner']['files']})!=len(v['runner']['files']):raise ValueError('runner_duplicate')
    exact(v['powershell_images'],('system32','syswow64'))
    for a in v['powershell_images'].values():
        exact(a,('sha256','size'));sha(a['sha256'])
        if type(a['size'])!=int or a['size']<=0:raise ValueError('image_size')
    exact(v['installed'],('record_sha256','artifacts'));sha(v['installed']['record_sha256']);exact(v['installed']['artifacts'],OWNERS)
    for a in v['installed']['artifacts'].values():art(a)
    exact(v['sources'],SOURCES)
    for h in v['sources'].values():sha(h)
    sha(v['manifest_sha256']);exact(v['source'],('head','tree'))
    for h in v['source'].values():
        if not re.fullmatch('[a-f0-9]{40}',h):raise ValueError('source_git_identity')
    b=v['baseline_windows_environment'];exact(b,('present','utf16_code_units','sha256_utf16le','duplicate_count'));sha(b['sha256_utf16le'])
    if type(b['present'])!=bool or type(b['utf16_code_units'])!=int or not 0<=b['utf16_code_units']<=32767 or b['duplicate_count']!=0:raise ValueError('windows_baseline')

def envelope(manifest,seal):
    return {'schema':2,**{k:manifest[k] for k in ('payload','runner','powershell_images','installed','source','baseline_windows_environment')},'sources':{n:manifest['files'][n] for n in SOURCES},'manifest_sha256':seal}

def verify_package(root,seal,source):
    root=pathlib.Path(root);sha(seal)
    if root.is_symlink() or set(p.name for p in root.iterdir())!=REQUIRED|{MANIFEST}:raise ValueError('package_exact_file_set')
    p=root/MANIFEST
    raw=read_bytes(p)
    if hashlib.sha256(raw).hexdigest()!=seal:raise ValueError('manifest_seal')
    m=json.loads(raw,object_pairs_hook=no_duplicates);exact(m,('schema','files','payload','source','runner','powershell_images','installed','baseline_windows_environment','build'))
    if m['schema']!=2 or m['source']!=source:raise ValueError('manifest_generation')
    if raw!=canonical(m):raise ValueError('manifest_not_canonical')
    exact(m['files'],REQUIRED)
    exact(m['build'],('recipe','compiler_sha256','compiler_version','rustc_sha256','rustc_version'));sha(m['build']['compiler_sha256'])
    sha(m['build']['rustc_sha256'])
    if m['build']['recipe']!='is4-zig-x86-cpp20-rust-policy-v1' or len(m['build']['compiler_version'])>128:raise ValueError('build_recipe')
    for name,h in m['files'].items():
        sha(h);f=root/name;s=f.lstat()
        if not stat.S_ISREG(s.st_mode) or s.st_uid!=os.getuid() or s.st_mode&0o222 or digest(f)!=h:raise ValueError('sealed_file_drift')
    payload=root/'payload.exe'
    if m['payload']!={'sha256':digest(payload),'size':payload.stat().st_size,'architecture':architecture(payload)} or m['payload']['architecture']!='x86':raise ValueError('payload_manifest')
    validate_identity(envelope(m,seal))
    return m

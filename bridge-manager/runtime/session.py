#!/usr/bin/env python3
"""One installed instance: existing Windows host, bounded pipes and PID/start cleanup.

No registration writes, checkout imports, scan guesses, global Wine overrides or
callback work. The Rust manager supplies an exact verified registration.
"""
import ctypes,mmap,collections,re
import fcntl,hashlib,json,os,pathlib,pwd,selectors,shutil,signal,socket,stat,struct,subprocess,sys,time
from ownership import process_identities,descendant_identities,cleanup_process,ProcessTracker,CompanionCgroup,InstallerLedger

def atomic(path,value):
    temp=path.with_suffix(path.suffix+'.tmp')
    with temp.open('x') as f: json.dump(value,f);f.flush();os.fsync(f.fileno())
    temp.replace(path)

def verify(artifact):
    p=pathlib.Path(artifact['path']);m=p.lstat()
    if not stat.S_ISREG(m.st_mode) or m.st_uid!=os.getuid():raise RuntimeError('artifact ownership/type differs')
    with os.fdopen(os.open(p,os.O_RDONLY|os.O_NOFOLLOW),'rb') as f:
        opened=os.fstat(f.fileno())
        if (opened.st_dev,opened.st_ino)!=(m.st_dev,m.st_ino):raise RuntimeError('artifact replaced during verification')
        digest=hashlib.file_digest(f,'sha256').hexdigest()
    if digest!=artifact['sha256']:raise RuntimeError('registered artifact changed')

def windows(path,prefix):
    path=pathlib.Path(path)
    try:return 'C:\\'+str(path.relative_to(prefix/'drive_c')).replace('/','\\')
    except ValueError:return 'Z:'+str(path).replace('/','\\')

def environment(reg):
    root=pathlib.Path(reg['environment']['root']);user=pwd.getpwuid(os.getuid())
    env={'HOME':user.pw_dir,'USER':user.pw_name,'LOGNAME':user.pw_name,'PATH':'/usr/bin:/bin','LANG':'C.UTF-8',
         'XDG_RUNTIME_DIR':f'/run/user/{os.getuid()}','STEAM_COMPAT_DATA_PATH':str(root/'compatdata'),
         'STEAM_COMPAT_CLIENT_INSTALL_PATH':str(root/'client'),'STEAM_COMPAT_APP_ID':'0','SteamAppId':'0','SteamGameId':'0',
         'STEAM_ZENITY':'','PRESSURE_VESSEL_VARIABLE_DIR':str(root/'runtime-var')}
    for key,folder in [('XDG_CACHE_HOME','host-cache'),('XDG_CONFIG_HOME','host-config'),('XDG_DATA_HOME','host-data'),('TMPDIR','host-tmp')]:env[key]=str(root/folder)
    for line in subprocess.check_output(['systemctl','--user','show-environment'],text=True,timeout=5).splitlines():
        key,_,value=line.partition('=')
        if key in ('DISPLAY','XAUTHORITY','WAYLAND_DISPLAY','DBUS_SESSION_BUS_ADDRESS'):env[key]=value
    if not env.get('DISPLAY'):raise RuntimeError('graphical user session is unavailable')
    if reg['compatibility']['disable_windows_accessibility']:env['WINEDLLOVERRIDES']='uiautomationcore='
    policy=reg['compatibility'].get('event_output')
    if policy is not None:
        if policy!='reported_zero_event_channels_unspecified':raise RuntimeError('unsupported event output policy')
        env['LVB_EVENT_OUTPUT_POLICY']=policy
    lifetime=reg['compatibility'].get('editor_lifetime')
    if lifetime is not None:
        if lifetime!='retain_editor_view_until_instance_retirement':raise RuntimeError('unsupported editor lifetime')
        env['LVB_EDITOR_LIFETIME']=lifetime
    retirement=reg['compatibility'].get('vendor_retirement')
    if retirement is not None:
        if retirement!='process_scoped_vendor_retirement' or lifetime!='retain_editor_view_until_instance_retirement':raise RuntimeError('unsupported vendor retirement')
        env['LVB_VENDOR_RETIREMENT']=retirement
    return env

def managed_home(spec,env):
    # The Rust owner verifies the exact onboarding/environment record before
    # setting this flag. Keep the installed machine's HOME for its keeper,
    # inspection and candidate DSP/editor alike; never copy the operator HOME.
    if spec.get('onboarding_home'):
        home=pathlib.Path(spec['registration']['environment']['root'])/'home'
        private_directory(home)
        env['HOME']=str(home)

def command(spec):
    reg=spec['registration'];root=pathlib.Path(reg['environment']['root']);prefix=root/'compatdata/pfx';runner=reg['environment']['runner'];sid=spec['session'];mode='ap12-vendor-access' if spec.get('vendor_access') else 'ap8-module-inspection' if spec['inspect'] else 'ap9-commercial'
    if spec.get('bus_lifecycle_probe'):
        if spec['inspect'] is not True or spec.get('keeper') or spec.get('vendor_access'):
            raise RuntimeError('bus lifecycle probe requires isolated inspection')
        mode='ap18-bus-lifecycle'
    case='first-audio' if spec.get('first_audio') else 'class:'+reg['metadata']['class_id']
    handshake_directory=prefix/'drive_c/bridge/sessions'/sid
    pairs=[('session',sid),('scanner-sha256',reg['host']['sha256']),('implementation-source-manifest-sha256',reg['host_source_sha256']),
           ('module',windows(reg['module']['path'],prefix)),('module-sha256',reg['module']['sha256']),('bundle-manifest-sha256',reg['module']['sha256']),
           ('ready',windows(handshake_directory/(sid+'.ready'),prefix)),('gate',windows(handshake_directory/(sid+'.gate'),prefix)),
           ('max-classes','256'),('stdout-cap','1048576'),('mode',mode),('component-case',case)]
    cmd=[runner['entry_point'],'--verb=run','--',runner['proton'],'runinprefix',windows(reg['host']['path'],prefix)]
    for k,v in pairs:cmd+=['--'+k,v]
    binding=('schema=linux-vst-bridge-wf0-handshake/v1\n'+''.join(k.replace('-','_')+'='+v+'\n' for k,v in [pairs[0],pairs[1],pairs[4],pairs[5],pairs[2],pairs[10],pairs[11]])+'run_ordinal=1\n').encode()
    return cmd,binding


# AP16: the hot mappings share memory, while owner.json and retirement receipts
# retain their existing durable locations. Only setup/admission/retirement use
# these filesystem checks; no sampler or storage work is added to a callback.
STORAGE_MARKER=b'linux-vst-bridge volatile transport v1\n'
def transport_root():return pathlib.Path('/run/user')/str(os.getuid())/'linux-vst-bridge'
def private_directory(path):
    m=path.lstat()
    if not stat.S_ISDIR(m.st_mode) or m.st_uid!=os.getuid() or m.st_mode&0o077 or path.resolve()!=path:
        raise RuntimeError('transport directory ownership/alias')
    return m

def memory_directory(path):
    m=private_directory(path)
    fd=os.open(path,os.O_RDONLY|os.O_DIRECTORY|os.O_NOFOLLOW)
    try:
        opened=os.fstat(fd)
        if (m.st_dev,m.st_ino)!=(opened.st_dev,opened.st_ino):raise RuntimeError('transport directory replaced')
        # Linux statfs begins with a signed long f_type. The bounded buffer is
        # larger than statfs on the supported x86_64 Linux runtime. No shell.
        lib=ctypes.CDLL(None,use_errno=True);buf=ctypes.create_string_buffer(256)
        lib.fstatfs.argtypes=[ctypes.c_int,ctypes.c_void_p];lib.fstatfs.restype=ctypes.c_int
        if lib.fstatfs(fd,ctypes.byref(buf))!=0:raise OSError(ctypes.get_errno(),'transport statfs')
        if ctypes.c_long.from_buffer(buf).value!=0x01021994:raise RuntimeError('transport requires tmpfs')
    finally:os.close(fd)
    return m

def validate_runtime():
    root=transport_root();memory_directory(root)
    marker=root/'storage-v1';m=marker.lstat()
    if not stat.S_ISREG(m.st_mode) or m.st_uid!=os.getuid() or m.st_mode&0o077 or m.st_size!=len(STORAGE_MARKER):
        raise RuntimeError('transport root foreign')
    with os.fdopen(os.open(marker,os.O_RDONLY|os.O_NOFOLLOW),'rb') as f:
        if f.read(len(STORAGE_MARKER)+1)!=STORAGE_MARKER:raise RuntimeError('transport root foreign')
    return root

def session_directories(spec):
    sid=spec['session'];directory=pathlib.Path(spec['directory'])
    durable=pathlib.Path(spec['registration']['environment']['root'])/'compatdata/pfx/drive_c/bridge/sessions'/sid
    if len(sid)!=32 or any(c not in '0123456789abcdef' for c in sid):raise RuntimeError('session binding differs')
    private_directory(durable)
    transport=spec.get('transport')
    if transport is None:
        if directory!=durable:raise RuntimeError('session binding differs')
    else:
        if not isinstance(transport,dict) or set(transport)!={'schema','device','inode'} or any(type(v) is not int for v in transport.values()) or transport['schema']!=1 or spec.get('shared_runtime') is not True or spec['inspect'] or spec.get('keeper') or spec.get('vendor_access'):
            raise RuntimeError('transport ownership/version')
        root=validate_runtime()
        if directory!=root/sid:raise RuntimeError('transport session binding differs')
        m=memory_directory(directory)
        if (m.st_dev,m.st_ino)!=(transport['device'],transport['inode']):raise RuntimeError('transport directory replaced')
    return directory,durable

def transport_environment(spec,env):
    if spec.get('shared_runtime'):
        # The keeper and every audio child see only our same private root.
        # This is not a prefix, home or arbitrary caller-selected mount grant.
        env['PRESSURE_VESSEL_FILESYSTEMS_RW']=str(validate_runtime())

def windows_transport_views(spec):
    """Keep the pinned host's closed C: handshake contract. Only these session-owner
    created files alias the exact RAM session. Publish before the Windows gate,
    never during processing; owner.json/handshake/receipts remain durable.
    """
    directory,durable=session_directories(spec)
    if directory==durable:return
    sources=[]
    for name in ('ap1.control','ap1.audio','ap11.ui','ap12.status','ap10.delivery','ap18.results','ap18.retirement','if1.terminal'):
        source=directory/name;target=durable/name
        try:m=source.lstat()
        except FileNotFoundError:
            if name=='ap18.retirement' and spec.get('registration',{}).get('compatibility',{}).get('vendor_retirement') is None:continue
            if name in ('ap10.delivery','ap18.results','if1.terminal'):continue # retained legacy diagnostic clients
            raise
        if not stat.S_ISREG(m.st_mode) or m.st_uid!=os.getuid() or m.st_mode&0o077:
            raise RuntimeError('transport file ownership/type')
        if os.path.lexists(target):raise RuntimeError('Windows transport view already exists')
        sources.append((source,target,m))
    for source,target,m in sources:
        target.symlink_to(source)
        visible=target.stat()
        if (visible.st_dev,visible.st_ino)!=(m.st_dev,m.st_ino):
            raise RuntimeError('Windows transport view replaced')

def retire_directories(spec):
    directory,durable=session_directories(spec)
    shutil.rmtree(directory)
    if directory!=durable:shutil.rmtree(durable)

def delivery_trace(spec,env):
    # Match the registered native observer's opt-in flag. The supervisor's
    # deliberately small environment must not drop the Windows half of a trace.
    # This is read once before launch, never by either audio delivery thread.
    if spec['inspect'] or spec.get('vendor_access'):return
    flag=pathlib.Path(env['HOME'])/'.local/share/linux-vst-bridge/managed/runtime/trace-enable'
    try:
        with flag.open('rb') as f:enabled=f.read(3)==b'1\n'
    except OSError:enabled=False
    if enabled:env['LVB_AP10_TRACE']='1'

def operation_lock_mode(spec):
    # Shared inspection is admitted by the existing Rust service only after it
    # owns the environment keeper and excludes all DSP admissions for the scan.
    # Standalone/internal inspect remains exclusive.
    if spec.get('shared_inspection'):
        if not spec['inspect'] or spec.get('keeper') or spec.get('vendor_access'):
            raise RuntimeError('invalid shared inspection ownership')
        return fcntl.LOCK_SH
    return fcntl.LOCK_EX if spec['inspect'] and not spec.get('keeper') else fcntl.LOCK_SH

class ResultStatus:
    """LVRS v1 first-rejection custody, independently readable after host death.

    Fixed 1024-byte file, two 384-byte slots and an atomic commit. No vendor
    payloads. All concurrent words use libatomic, never Python memory copies.
    """
    fields=('generation','epoch','request_sequence','position','callback','input_notes','input_parameters',
            'reason','frames','events','points','queues','bytes','event_type','bus','offset','channel',
            'declared_channels','event_bus_active','flags','declared_buses','payload_type','payload_size',
            'parameter_id','ppq_bits','value_bits','field_a','field_b','extra_bits','process_returned','reserved')
    reasons=('None','EventCapacity','NegativeBus','UndeclaredBus','BusStorage','NegativeEventOffset',
             'EventExtent','NonFinitePPQ','UnsupportedEvent','InvalidEventField','EventChannel','EventPayload',
             'PayloadCapacity','NullPayload','PayloadAlignment','QueueCapacity','PointCapacity',
             'NegativePointOffset','PointExtent','NonFiniteValue','ValueBelowZero','ValueAboveOne')
    @staticmethod
    def create(directory,sid):
        header=b'LVRS'+struct.pack('<III',1,1024,31)+bytes.fromhex(sid)
        fd=os.open(directory/'ap18.results',os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW,0o600)
        with os.fdopen(fd,'wb') as f:
            f.write(header+bytes(1024-len(header)))
    def __init__(self,directory,sid):
        self.map=None;self.sid=sid;self.last=None
        try:fd=os.open(directory/'ap18.results',os.O_RDWR|os.O_NOFOLLOW)
        except FileNotFoundError:return
        try:
            st=os.fstat(fd)
            if not stat.S_ISREG(st.st_mode) or st.st_uid!=os.getuid() or st.st_size!=1024 or st.st_mode&0o077:raise RuntimeError('result status ownership/extent')
            self.map=mmap.mmap(fd,1024,access=mmap.ACCESS_WRITE)
        finally:os.close(fd)
        if self.map[:32]!=b'LVRS'+struct.pack('<III',1,1024,31)+bytes.fromhex(sid):
            self.close();raise RuntimeError('result status identity/version')
        lib=ctypes.CDLL('libatomic.so.1');self.load=getattr(lib,'__atomic_load_8')
        self.load.argtypes=[ctypes.c_void_p,ctypes.c_int];self.load.restype=ctypes.c_uint64
        self.address=ctypes.addressof(ctypes.c_char.from_buffer(self.map))
    def word(self,at):return self.load(self.address+at,5)
    def snapshot(self):
        if self.map is None:return {'available':False}
        for _ in range(3):
            commit=self.word(64)
            if not commit:return {'available':True,'schema':1,'rejection':None}
            if commit!=1:raise RuntimeError('result status first-write counter')
            row=dict(zip(self.fields,(self.word(128+(commit&1)*384+i*8) for i in range(31))))
            if commit!=self.word(64):continue
            if not 0<row['reason']<len(self.reasons) or row['process_returned']!=1 or row['reserved']:raise RuntimeError('result status malformed record')
            for key in ('frames','event_type','bus','offset','channel','declared_channels','event_bus_active','field_a','field_b'):
                v=row[key]
                if v>0xffffffff:raise RuntimeError('result status scalar extent')
                row[key]=v-(1<<32) if v&(1<<31) else v
            row['reason_code']=row['reason'];row['reason']=self.reasons[row['reason']]
            self.last={'available':True,'schema':1,'session':self.sid,'publication':commit,'rejection':row}
            return self.last
        return self.last or {'available':True,'schema':1,'incomplete':True}
    def close(self):
        if self.map is not None:self.map.close();self.map=None

class RetirementStatus:
    """LVRT v1: required, session-bound final process retirement authority.

    Header is immutable; eight atomic scalar words occupy an inactive slot.
    Commit one authorizes containment only after all seven milestones. A
    partial write or an ordinary host exit is never positive retirement.
    """
    fields=('milestones','epoch','sequence','position','generation','reserved0','reserved1','reserved2')
    @staticmethod
    def header(sid):return b'LVRT'+struct.pack('<III',1,256,8)+bytes.fromhex(sid)
    @classmethod
    def create(cls,directory,sid):
        fd=os.open(directory/'ap18.retirement',os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW,0o600)
        with os.fdopen(fd,'wb') as f:f.write(cls.header(sid)+bytes(224))
    def __init__(self,directory,sid):
        self.map=None;self.sid=sid
        fd=os.open(directory/'ap18.retirement',os.O_RDWR|os.O_NOFOLLOW)
        try:
            st=os.fstat(fd)
            if not stat.S_ISREG(st.st_mode) or st.st_uid!=os.getuid() or st.st_size!=256 or st.st_mode&0o077:raise RuntimeError('retirement status ownership/extent')
            self.map=mmap.mmap(fd,256,access=mmap.ACCESS_WRITE)
        finally:os.close(fd)
        if self.map[:32]!=self.header(sid):
            self.close();raise RuntimeError('retirement status identity/version')
        lib=ctypes.CDLL('libatomic.so.1');self.load=getattr(lib,'__atomic_load_8')
        self.load.argtypes=[ctypes.c_void_p,ctypes.c_int];self.load.restype=ctypes.c_uint64
        self.address=ctypes.addressof(ctypes.c_char.from_buffer(self.map))
    def word(self,at):return self.load(self.address+at,5)
    def snapshot(self):
        commit=self.word(64)
        if not commit:return None
        if commit!=1:raise RuntimeError('retirement status commit')
        row=dict(zip(self.fields,(self.word(192+i*8) for i in range(8))))
        if self.word(64)!=commit:raise RuntimeError('retirement status changed')
        if row['milestones']!=127 or any(row[k] for k in ('reserved0','reserved1','reserved2')):raise RuntimeError('retirement status incomplete')
        return dict(session=self.sid,schema=1,state='process_scoped_retirement_ready',**row)
    def close(self):
        if self.map is not None:self.map.close();self.map=None

class TerminalStatus:
    """IF1 v1: native-created custody, separate fixed single-writer slots.

    A complete producer slot is committed with libatomic compare/exchange.
    The supervisor never claims a slot before writing it and never replaces
    another owner's first failure. Optional only for older immutable natives.
    """
    def __init__(self,directory,sid):
        self.map=None;self.sid=sid;self.completed=False;self.pending=None
        try:fd=os.open(directory/'if1.terminal',os.O_RDWR|os.O_NOFOLLOW)
        except FileNotFoundError:return
        try:
            st=os.fstat(fd)
            if not stat.S_ISREG(st.st_mode) or st.st_uid!=os.getuid() or st.st_size!=2048 or st.st_mode&0o077:raise RuntimeError('IF1 ownership/extent')
            self.map=mmap.mmap(fd,2048,access=mmap.ACCESS_WRITE)
        finally:os.close(fd)
        if self.map[:32]!=b'LVIF'+struct.pack('<III',1,2048,0)+bytes.fromhex(sid):
            self.close();raise RuntimeError('IF1 session/version')
        self.lib=ctypes.CDLL('libatomic.so.1')
        self.load=getattr(self.lib,'__atomic_load_8');self.load.argtypes=[ctypes.c_void_p,ctypes.c_int];self.load.restype=ctypes.c_uint64
        self.store=getattr(self.lib,'__atomic_store_8');self.store.argtypes=[ctypes.c_void_p,ctypes.c_uint64,ctypes.c_int]
        self.cas=getattr(self.lib,'__atomic_compare_exchange_8');self.cas.argtypes=[ctypes.c_void_p,ctypes.POINTER(ctypes.c_uint64),ctypes.c_uint64,ctypes.c_bool,ctypes.c_int,ctypes.c_int];self.cas.restype=ctypes.c_bool
        self.address=ctypes.addressof(ctypes.c_char.from_buffer(self.map))
    def word(self,o):return self.load(self.address+o,5)
    def root_exit(self,status,domain=4):
        if self.map is None or self.completed:return True
        # Preserve the first observed failure across bounded unstable reads.
        # Only a complete competing record or a completed CAS ends custody.
        if self.pending is None:self.pending=(status,domain)
        if self.word(64):self.completed=True;return True
        status,domain=self.pending
        for _ in range(3):
            c=self.word(1024)
            if not c:return False
            base=1088+(c&1)*256
            row=[self.word(base+i*8) for i in range(24)]
            if self.word(1024)!=c:continue
            row[14:16]=[1,status&0xffffffffffffffff];row[18]=3;row[19]=domain
            for i,v in enumerate(row):self.store(self.address+640+i*8,v,5)
            expected=ctypes.c_uint64(0);self.cas(self.address+64,ctypes.byref(expected),3,False,5,5)
            self.completed=True
            return True
        return False  # retry on the next owner call; never consume pending failure
    def snapshot(self):
        if self.map is None:return None
        c=self.word(64)
        if not c:return None
        if c not in (1,2,3):raise RuntimeError('IF1 producer')
        r=[self.word(128+(c-1)*256+i*8) for i in range(24)]
        if r[0]!=1 or struct.pack('<QQ',*r[1:3])!=bytes.fromhex(self.sid) or r[18]!=c or r[14] not in (1,2,3) or r[19] not in (1,2,3,4,5) or any(r[20:]):raise RuntimeError('IF1 record identity')
        return {'schema':1,'session':self.sid,'generation':r[3],'epoch':r[4],'sequence':r[5],
          'last_completed_position':r[6],'state_revision':r[7],'state_generation':r[8],'state_through':r[9],
          'state_sha256':struct.pack('<QQQQ',*r[10:14]).hex(),'failure_class':r[14],
          'status':r[15],'position':r[16],'native_phase':r[17],'producer':r[18],'status_domain':r[19]}
    def close(self):
        if self.map is not None:self.map.close();self.map=None

class FaultStatus:
    """Atomic, bounded read of AP12 status; independent of either Windows thread.

    All words are atomic, including slot data. A stable publication counter
    validates a slot. Three attempts per lane, never wait for a writer. The
    inactive slot preserves the last complete publication if a writer is killed.
    Linux libatomic supplies acquire/SC reads; Python byte copies are NOT used
    for concurrently written slots. Header bytes are immutable before admission.
    """
    fields=('generation','epoch','request_sequence','position','stage','detail','ticks','frequency','thread_id','process_id')
    def __init__(self,directory,sid):
        self.terminal=TerminalStatus(directory,sid);self.result_status=ResultStatus(directory,sid);self.map=None;self.mailbox=None;self.gui=None;self.directory=directory;self.sid=sid;self.last=[None]*3;self.pending=None;self.suspect=None
        try:
            fd=os.open(directory/'ap12.status',os.O_RDWR|os.O_NOFOLLOW)
        except FileNotFoundError:return # legacy diagnostic clients
        try:
            st=os.fstat(fd)
            if not stat.S_ISREG(st.st_mode) or st.st_uid!=os.getuid() or st.st_size!=1024 or st.st_mode&0o077:raise RuntimeError('fault status ownership/extent')
            self.map=mmap.mmap(fd,1024,access=mmap.ACCESS_WRITE)
        finally:os.close(fd)
        header=self.map[:32]
        self.version=struct.unpack_from('<I',header,4)[0]
        if self.version not in (1,2) or header[:16]!=b'LVFS'+struct.pack('<III',self.version,1024,0) or header[16:]!=bytes.fromhex(sid):
            self.close();raise RuntimeError('fault status session/version')
        self.lib=ctypes.CDLL('libatomic.so.1')
        self.load=getattr(self.lib,'__atomic_load_8');self.load.argtypes=[ctypes.c_void_p,ctypes.c_int];self.load.restype=ctypes.c_uint64
        self.address=ctypes.addressof(ctypes.c_char.from_buffer(self.map))
        self.load4=getattr(self.lib,'__atomic_load_4');self.load4.argtypes=[ctypes.c_void_p,ctypes.c_int];self.load4.restype=ctypes.c_uint32
        try:fd=os.open(directory/'ap10.delivery',os.O_RDWR|os.O_NOFOLLOW)
        except FileNotFoundError:return
        try:
            st=os.fstat(fd)
            if not stat.S_ISREG(st.st_mode) or st.st_uid!=os.getuid() or st.st_size!=33024 or st.st_mode&0o077:raise RuntimeError('fault mailbox ownership/extent')
            self.mailbox=mmap.mmap(fd,33024,access=mmap.ACCESS_WRITE)
            version=struct.unpack_from('<I',self.mailbox,4)[0]
            if version not in (2,3) or self.mailbox[:32]!=b'LVBM'+struct.pack('<III',version,33024,0)+bytes.fromhex(sid):raise RuntimeError('fault mailbox identity/version')
            self.mailbox_address=ctypes.addressof(ctypes.c_char.from_buffer(self.mailbox))
        except Exception:self.close();raise
        finally:os.close(fd)
    def word(self,offset):return self.load(self.address+offset,5) # sequentially consistent
    def lane(self,index):
        base=64+index*320
        for _ in range(3):
            counter=self.word(base)
            if not counter:return None
            slot=base+64+(counter&1)*128
            fields=self.fields + (('admitted_frames','missing_frames','gaps','expired_frames','delivered_frames','priming_frames') if index==0 and self.version==2 else ())
            values=[self.word(slot+8*i) for i in range(len(fields))]
            if self.word(base)==counter:
                self.last[index]={'publication':counter,**dict(zip(fields,values))}
                return {**self.last[index],'current':True}
        return {**self.last[index],'current':False} if self.last[index] else {'current':False}
    def snapshot(self):
        if self.map is None:return {'available':False,'result_status':self.result_status.snapshot(),'terminal_instance':self.terminal.snapshot()}
        return {'available':True,'schema':self.version,'sample_monotonic_ns':time.monotonic_ns(),
                'clock_domains':['linux_monotonic_ns','windows_qpc','windows_qpc'],
                **{name:self.lane(i) for i,name in enumerate(('native','delivery','owner'))},
                'editor':self.editor_snapshot(),
                'result_status':self.result_status.snapshot(),'terminal_instance':self.terminal.snapshot(),
                'mailbox_flags':None if self.mailbox is None else {
                    'request':self.load4(self.mailbox_address+64,5),
                    'reply':self.load4(self.mailbox_address+128,5),
                    'independently_sampled':True}}
    def editor_snapshot(self):
        # Existing UI header only: never copy parameter/state/event payloads.
        # Open lazily because the UI mapping may follow supervisor admission.
        if self.gui is None:
            try:fd=os.open(self.directory/'ap11.ui',os.O_RDWR|os.O_NOFOLLOW)
            except FileNotFoundError:return {'available':False}
            try:
                st=os.fstat(fd)
                if not stat.S_ISREG(st.st_mode) or st.st_uid!=os.getuid() or st.st_size<256 or st.st_mode&0o077:raise RuntimeError('fault GUI ownership/extent')
                self.gui=mmap.mmap(fd,256,access=mmap.ACCESS_WRITE)
                version,extent,message=struct.unpack_from('<III',self.gui,4)
                # Exact retained AP14 and AP15 layouts. Header diagnostics have
                # identical offsets; event payloads are never read by the owner.
                layouts={3:(256+2*512*584,584),4:(320+2*512*608,608),5:(320+2*512*608,608)}
                if self.gui[:4]!=b'LVBU' or layouts.get(version)!=(extent,message) or st.st_size!=extent or self.gui[16:32]!=bytes.fromhex(self.sid) or self.gui[32:36]!=struct.pack('<I',512):raise RuntimeError('fault GUI identity/version')
                self.gui_address=ctypes.addressof(ctypes.c_char.from_buffer(self.gui))
            except Exception:
                if self.gui is not None:self.gui.close();self.gui=None
                raise
            finally:os.close(fd)
        # These existing scalars have individual atomic publication, not a
        # joint transaction. Exception code is published after its metadata.
        fields={'closed':104,'failure':108,'open':116,'view_stage':160,'open_stage':164,'exception_code':168}
        result={'available':True,'independently_sampled':True,**{k:self.load4(self.gui_address+o,5) for k,o in fields.items()}}
        if result['exception_code']:
            result['exception_instruction']=self.load(self.gui_address+176,5)
        return result
    def poll(self):
        if self.map is None or self.suspect is not None:return
        native=self.lane(0)
        if not native or not native.get('current') or native['stage'] not in (1,2):self.pending=None;return
        identity=(native['generation'],native['epoch'],native['request_sequence'])
        now=time.monotonic()
        if self.pending is None or self.pending[0]!=identity:self.pending=(identity,now)
        elif now-self.pending[1]>=1:
            # One bounded early witness; does not change the five-second reply
            # deadline or declare that this request necessarily fails later.
            self.suspect=self.snapshot();self.suspect['observed_pending_seconds']=now-self.pending[1]
    def close(self):
        self.terminal.close()
        self.result_status.close()
        if self.gui is not None:self.gui.close();self.gui=None
        if self.mailbox is not None:self.mailbox.close();self.mailbox=None
        if self.map is not None:self.map.close();self.map=None

def fault_threads(owned):
    """One bounded /proc observation only after a request stays pending >1s."""
    result=[]
    current={(p['pid'],p['start_ticks']) for p in process_identities()}
    for pid,start in sorted(owned):
        if (pid,start) not in current:continue
        try:tasks=sorted((pathlib.Path('/proc')/str(pid)/'task').iterdir())
        except OSError:continue
        for task in tasks:
            if len(result)>=128:return result
            try:
                raw=(task/'stat').read_text();end=raw.rfind(')');fields=raw[end+2:].split()
                wchan=(task/'wchan').read_text()[:128]
                result.append({'pid':pid,'start_ticks':start,'tid':int(task.name),'name':raw[raw.find('(')+1:end][:64],
                               'state':fields[0],'utime':int(fields[11]),'stime':int(fields[12]),'wchan':wchan})
            except (OSError,ValueError,IndexError):continue
    return result

# CA1 is a supervisor-side observer. None of this code runs on an audio or
# Windows vendor thread. Its errors never authorize or prevent cleanup.
class RecentCapture:
    """Record-bounded tail, unlike ASC's first-N PrivateCapture."""
    def __init__(self, capacity, records):
        self.capacity=capacity;self.records=records;self.rows=collections.deque();self.bytes=0;self.dropped_bytes=0;self.dropped_records=0
    def write(self, data):
        if len(data)>self.capacity:
            self.dropped_bytes+=len(data);self.dropped_records+=1;return
        while self.rows and (self.bytes+len(data)>self.capacity or len(self.rows)>=self.records):
            old=self.rows.popleft();self.bytes-=len(old);self.dropped_bytes+=len(old);self.dropped_records+=1
        self.rows.append(data);self.bytes+=len(data)
    def value(self):return [x.decode('utf-8',errors='replace') for x in self.rows]
    def limits(self):return dict(capacity=self.capacity,record_capacity=self.records,retained_bytes=self.bytes,dropped_bytes=self.dropped_bytes,dropped_records=self.dropped_records)


def capture_pe(f):
    """Parse the same opened file used for identity verification and hashing."""
    size=os.fstat(f.fileno()).st_size
    def at(offset,n):
        if offset<0 or n>262144 or offset+n>size:raise ValueError('PE range')
        f.seek(offset);b=f.read(n)
        if len(b)!=n:raise ValueError('PE truncated')
        return b
    dos=at(0,64)
    if dos[:2]!=b'MZ':raise ValueError('not PE')
    pos=struct.unpack_from('<I',dos,60)[0];head=at(pos,24)
    if head[:4]!=b'PE\0\0':raise ValueError('not PE')
    count=struct.unpack_from('<H',head,6)[0];opt_size=struct.unpack_from('<H',head,20)[0]
    if count>96 or opt_size>512:raise ValueError('PE metadata capacity')
    opt=at(pos+24,opt_size);magic=struct.unpack_from('<H',opt)[0]
    if magic not in (0x10b,0x20b):raise ValueError('PE optional header')
    image_size=struct.unpack_from('<I',opt,56)[0];sections=at(pos+24+opt_size,count*40)
    def rva(v,n):
        for i in range(count):
            vs,va,raw,off=struct.unpack_from('<IIII',sections,i*40+8)
            if va<=v and v+n<=va+min(vs,raw):return at(off+v-va,n)
        raise ValueError('PE RVA unmapped')
    exports=[];directory=112 if magic==0x20b else 96
    er,es=struct.unpack_from('<II',opt,directory)
    if er and es:
        eh=rva(er,40);nf,nn,af,an,ao=struct.unpack_from('<IIIII',eh,20)
        if nf>8192 or nn>8192:raise ValueError('PE export capacity')
        funcs=rva(af,nf*4);names=rva(an,nn*4);ordinals=rva(ao,nn*2)
        for i in range(nn):
            idx=struct.unpack_from('<H',ordinals,i*2)[0]
            if idx>=nf:raise ValueError('PE ordinal')
            nr=struct.unpack_from('<I',names,i*4)[0]
            name=bytearray()
            for j in range(128):
                c=rva(nr+j,1)
                if c==b'\0':break
                name.extend(c)
            else:continue
            address=struct.unpack_from('<I',funcs,idx*4)[0]
            if er<=address<er+es:continue # forwarded export is not code
            if name and all(32<=x<127 for x in name):exports.append((address,name.decode('ascii')))
    return dict(size_of_image=image_size,exports=sorted(exports))


class IncidentCapture:
    LINE=8192;MODULES=1024;EXCEPTIONS=16;FRAMES=64;PROCESSES=128;SECONDS=7200;FINAL_SECONDS=5
    HEADER=re.compile(r'^(\d+\.\d+):([0-9a-fA-F]+):([0-9a-fA-F]+):(trace|warn|err):([a-z0-9_]+):([^ ]+) ?(.*)$')
    def __init__(self, spec, peer=None):
        self.request=spec['crash_capture'];self.path=pathlib.Path(self.request['directory']);self.sid=spec['session'];self.reg=spec['registration']
        if self.request.get('schema')!=1 or self.request.get('session')!=self.sid or not re.fullmatch('[0-9a-f]{32}',self.request.get('id','')):raise ValueError('capture binding')
        private_directory(self.path)
        if self.path.name!=self.request['id']:raise ValueError('capture directory')
        self.started=time.monotonic();self.active=True;self.finished=False;self.error=None;self.next_check=0;self.stop_reason=None
        self.context=RecentCapture(131072,512);self.terminal=RecentCapture(262144,1024)
        self.pending={};self.skipping={};self.modules=[];self.exceptions=collections.deque(maxlen=self.EXCEPTIONS);self.current={};self.exits={};self.fatal_reserved=[];self.processes={};self.images={};self.dirty=True
        self.counts=dict(received_bytes=0,excluded_bytes=0,oversized_lines=0,malformed_lines=0,module_overflow=0,exception_overwrite=0,frame_overflow=0,process_overflow=0,mapped_path_drops=0,incomplete_lines=0,disabled_bytes=0,final_drain_incomplete=False)
        self.native=None;self.launcher=None;self.first_terminal=None;self.map_bytes=0
        if peer is not None:
            try:
                pid,uid,_=struct.unpack('3i',peer.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))
                if uid==os.getuid():self.native=self.identity(pid)
            except (OSError,ValueError,AttributeError):pass
        self.status('collecting')
    def status(self,state):
        atomic(self.path/'status.json',dict(schema=1,state=state,session=self.sid,capture_enabled=self.active,error=self.error,logging_in_child=not self.finished))
    @staticmethod
    def identity(pid):
        p=pathlib.Path('/proc')/str(pid)
        try:
            raw=(p/'stat').read_text();fields=raw.rsplit(')',1)[1].split()
            return dict(pid=pid,start_ticks=int(fields[19]),ppid=int(fields[1]),pgrp=int(fields[2]),session=int(fields[3]))
        except (OSError,ValueError,IndexError):return None
    def check(self):
        if time.monotonic()<self.next_check:return
        self.next_check=time.monotonic()+1
        if self.active and ((self.path/'cancel.json').exists() or time.monotonic()-self.started>self.SECONDS):
            self.active=False;self.stop_reason='disarmed' if (self.path/'cancel.json').exists() else 'retention_expired';self.status(self.stop_reason)
    def write(self, stream, data):
        self.counts['received_bytes']+=len(data)
        if not self.active:self.counts['disabled_bytes']+=len(data);return
        if stream not in ('stderr','vendor'):raise ValueError('capture stream')
        # Parse bounded lines without retaining a giant unterminated record.
        for fragment in data.splitlines(keepends=True):
            if self.skipping.get(stream):
                self.counts['excluded_bytes']+=len(fragment)
                if fragment.endswith(b'\n'):self.skipping[stream]=False
                continue
            buf=self.pending.setdefault(stream,bytearray());buf.extend(fragment)
            if len(buf)>self.LINE:
                self.counts['oversized_lines']+=1;self.counts['excluded_bytes']+=len(buf);buf.clear();self.skipping[stream]=not fragment.endswith(b'\n');continue
            if fragment.endswith(b'\n'):
                line=bytes(buf);buf.clear();self.line(line)
    def line(self,line):
        text=line.decode('utf-8',errors='replace').rstrip('\r\n');m=self.HEADER.fullmatch(text)
        if not m:
            self.counts['excluded_bytes']+=len(line);self.counts['malformed_lines']+=1;return
        timestamp,pid,tid,level,channel,func,body=m.groups();pid=int(pid,16);tid=int(tid,16)
        when=dict(clock_domain='wine_trace_seconds',timestamp=timestamp,windows_pid=pid,windows_tid=tid)
        load=re.fullmatch(r'Loaded L"(.{1,1024})" at ([0-9a-fA-F]+): (builtin|native)',body) if channel=='loaddll' else None
        if load:
            if len(self.modules)>=self.MODULES:self.counts['module_overflow']+=1;return
            path,base,kind=load.groups();self.modules.append(dict(**when,path=path,base=int(base,16),kind=kind,unloaded=False));self.dirty=True;return
        if channel=='loaddll' and ('Unload' in body or 'unload' in func):
            self.context.write(line)
            unloaded=re.fullmatch(r'Unloaded module L"(.{1,1024})" : (builtin|native)',body)
            def windows_path(value):return value.replace('\\\\','\\').casefold()
            for mod in self.modules:
                if mod['windows_pid']!=pid or mod.get('unloaded_at'):continue
                if unloaded:
                    if windows_path(mod['path'])==windows_path(unloaded[1]):mod['unloaded_at']=timestamp
                elif not mod.get('unload_uncertain_at'):
                    mod['unload_uncertain_at']=timestamp
            # Retain load history. An unrelated or later unload does not erase
            # the actual image binding that existed at an earlier exception.
            return
        fault=re.search(r'code=([0-9a-fA-F]{8}) .*flags=([0-9a-fA-F]+) addr=([0-9a-fA-F]+)',body) if channel=='seh' and func=='dispatch_exception' else None
        if fault:
            if len(self.exceptions)==self.EXCEPTIONS:self.counts['exception_overwrite']+=1
            e=dict(**when,code=int(fault[1],16),flags=int(fault[2],16),ip=int(fault[3],16),access=None,address=None,frames=[],unhandled_marker=False)
            self.exceptions.append(e)
            # Avoid references to evicted exception rows growing unbounded.
            self.current={(x['windows_pid'],x['windows_tid']):x for x in self.exceptions}
            self.terminal.write(line);return
        e=self.current.get((pid,tid))
        if channel=='seh' and func=='dispatch_exception' and e:
            info=re.search(r'info\[([01])\]=([0-9a-fA-F]+)',body)
            if info:
                e['access' if info[1]=='0' else 'address']=int(info[2],16);self.terminal.write(line)
            # No registers/arguments/locals retained for diagnosis.
            return
        frame=re.search(r'type 1 base ([0-9a-fA-F]+) rip ([0-9a-fA-F]+) rva ([0-9a-fA-F]+)',body) if channel=='unwind' and func=='RtlVirtualUnwind2' else None
        if frame and e:
            if len(e['frames'])<self.FRAMES:e['frames'].append(dict(base=int(frame[1],16),ip=int(frame[2],16),rva=int(frame[3],16)))
            else:self.counts['frame_overflow']+=1
            self.terminal.write(line);return
        exit_match=re.search(r'handle (\(nil\)|0xffffffffffffffff|0xffffffff), exit_code (-?[0-9]+), process_exiting [01]',body) if channel=='process' and func=='NtTerminateProcess' else None
        if exit_match:
            code=int(exit_match[2],10)
            if not -(1<<31)<=code<(1<<32):self.counts['malformed_lines']+=1;return
            if pid in self.exits or len(self.exits)<self.PROCESSES:
                self.exits[pid]=dict(**when,exit_code=code&0xffffffff,raw_signed_exit_code=code,source='Wine NtTerminateProcess self pseudo-handle')
                if e and (code&0xffffffff)==e['code'] and e['code']>=0x80000000 and len(self.fatal_reserved)<4 and e not in self.fatal_reserved:
                    self.fatal_reserved.append(json.loads(json.dumps(e)))
            else:self.counts['process_overflow']+=1
            self.terminal.write(line);return
        if e and channel=='seh' and ('unhandled exception' in body.lower() or 'UnhandledExceptionFilter' in func):
            e['unhandled_marker']=True;self.terminal.write(line);return
        if (channel=='module' and level=='err') or (channel=='seh' and level in ('warn','err')):
            self.context.write(line);return
        self.counts['excluded_bytes']+=len(line)
    def observe(self,owned,root):
        self.check()
        if self.launcher is None:self.launcher=self.identity(root.pid)
        if not self.active:return
        # Existing ProcessTracker supplies identities; no new ownership authority.
        for pid,start in sorted(owned):
            key=(pid,start)
            if key in self.processes and not self.dirty:continue
            ident=self.identity(pid)
            if ident is None or ident['start_ticks']!=start:continue
            if key not in self.processes and len(self.processes)>=self.PROCESSES:self.counts['process_overflow']+=1;continue
            rec=self.processes.setdefault(key,dict(**ident,first_observed_monotonic_ns=time.monotonic_ns()))
            rec['last_observed_monotonic_ns']=time.monotonic_ns()
            try:
                rec['executable']=os.readlink(f'/proc/{pid}/exe')
                with open(f'/proc/{pid}/maps','rb') as f:data=f.read(4*1024*1024+1)
                if len(data)>4*1024*1024:raise ValueError('maps capacity')
                mapped={}
                for l in data.decode(errors='replace').splitlines():
                    fields=l.split(None,5)
                    if len(fields)!=6 or not fields[5].startswith('/'):continue
                    path=re.sub(r'\\([0-7]{3})',lambda m:chr(int(m[1],8)),fields[5])
                    if len(path)>2048:continue
                    major,minor=(int(x,16) for x in fields[3].split(':'));inode=int(fields[4])
                    deleted=path.endswith(' (deleted)')
                    if deleted:path=path[:-10]
                    resolved=None;unavailable=None
                    if deleted:unavailable='mapped file deleted'
                    elif not inode:unavailable='mapping has no file inode'
                    else:
                        try:resolved=str(pathlib.Path(path).resolve(strict=True))
                        except (OSError,RuntimeError):unavailable='mapped file path unavailable'
                    row=dict(path=path,resolved_path=resolved,device_major=major,device_minor=minor,inode=inode,
                             deleted=deleted,unavailable=unavailable,owner=dict(pid=pid,start_ticks=start))
                    mapped[json.dumps(row,sort_keys=True)]=row
                    if len(mapped)>1024:raise ValueError('mapped image capacity')
                # Retain observed identity history, including an old inode after
                # unlink/replacement. Paths never substitute for mapped bytes.
                retained=rec.setdefault('mapped_files',[])
                previous={json.dumps(x,sort_keys=True) for x in retained}
                for encoded,row in sorted(mapped.items()):
                    if encoded in previous:continue
                    n=len(encoded.encode())
                    if self.map_bytes+n>524288:self.counts['mapped_path_drops']+=1;continue
                    retained.append(row);self.map_bytes+=n
            except (OSError,ValueError) as err:rec['observation_error']=type(err).__name__
        self.dirty=False
    def retain_terminal(self,fault):
        value=(fault or {}).get('before_containment',{}).get('terminal_instance')
        if value is not None and self.first_terminal is None:
            if value.get('session')!=self.sid:raise ValueError('terminal session mismatch')
            self.first_terminal=json.loads(json.dumps(value))
    def resolve_modules(self):
        deadline=time.monotonic()+self.FINAL_SECONDS;hash_budget=512*1024*1024
        observed={}
        for r in self.processes.values():
            for row in r.get('mapped_files',[]):
                for path in {row['path'],row['resolved_path']}:
                    if path is not None:observed.setdefault(path,[]).append(row)
        prefix=pathlib.Path(self.reg['environment']['root'])/'compatdata/pfx'
        known={a['path']:a['sha256'] for a in [self.reg['host'],self.reg['module'],*self.reg['environment']['runner']['files']]}
        self.host_pids=set()
        for m in self.modules:
            win=m['path'].replace('\\\\','\\')
            location=pathlib.Path(win[2:].replace('\\','/')) if win[:2].lower()=='z:' else prefix/'drive_c'/win[3:].replace('\\','/') if win[:3].lower()=='c:\\' else None
            if location is None:continue
            # Do not resolve a possibly replaced symlink before O_NOFOLLOW.
            location=str(location)
            if location==self.reg['host']['path']:self.host_pids.add(m['windows_pid'])
            m['linux_path']=location
            if location not in self.images:
                info=dict(sha256=known.get(location),size_of_image=None,exports=[])
                try:
                    if time.monotonic()>=deadline:raise ValueError('module resolution deadline')
                    rows=observed.get(location,[])
                    if location not in known:
                        if not rows:raise ValueError('no exact observed Linux mapping')
                        if any(x['deleted'] or x['unavailable'] for x in rows):raise ValueError('mapped file deleted/unverifiable')
                    # O_NONBLOCK prevents a replaced FIFO from blocking cleanup;
                    # fstat refuses it before any read. Hash and parse one fd.
                    with os.fdopen(os.open(location,os.O_RDONLY|os.O_NOFOLLOW|os.O_NONBLOCK),'rb') as f:
                        before=os.fstat(f.fileno())
                        if not stat.S_ISREG(before.st_mode):raise ValueError('mapped file is not regular')
                        identity=(os.major(before.st_dev),os.minor(before.st_dev),before.st_ino)
                        if location not in known and any(identity!=(x['device_major'],x['device_minor'],x['inode']) for x in rows):
                            raise ValueError('mapped file replaced/unverifiable')
                        if before.st_size>hash_budget:raise ValueError('module hashing capacity')
                        sha=hashlib.file_digest(f,'sha256').hexdigest();hash_budget-=before.st_size
                        if location in known and sha!=known[location]:raise ValueError('registered artifact unavailable/changed')
                        parsed=capture_pe(f)
                        after=os.fstat(f.fileno())
                        def version(s):return (s.st_dev,s.st_ino,s.st_size,s.st_mtime_ns,s.st_ctime_ns)
                        if version(before)!=version(after):raise ValueError('mapped file changed during read')
                        info.update(parsed,sha256=sha)
                except (OSError,ValueError,struct.error) as err:
                    info['identity_unavailable']=str(err) if isinstance(err,ValueError) else 'mapped file deleted/unverifiable'
                self.images[location]=info
            info=self.images.get(location,{})
            m['sha256']=info.get('sha256');m['size_of_image']=info.get('size_of_image')
            if info.get('identity_unavailable'):m['identity_unavailable']=info['identity_unavailable']
        return deadline
    def frame(self,pid,ip,base=None,at=None):
        def current(m):
            if at is None:return not m.get('unloaded_at') and not m.get('unload_uncertain_at')
            return float(m['timestamp'])<=float(at) and all(not m.get(k) or float(m[k])>float(at) for k in ('unloaded_at','unload_uncertain_at'))
        choices=[m for m in self.modules if m['windows_pid']==pid and current(m) and m.get('size_of_image') and m['base']<=ip<m['base']+m['size_of_image'] and (base is None or base==m['base'])]
        if len(choices)!=1:return dict(ip=ip,module=None,offset=None,symbol=None,unavailable='loaded module missing or ambiguous')
        m=choices[0];offset=ip-m['base'];symbol=None
        exports=self.images.get(m.get('linux_path'),{}).get('exports',[])
        candidates=[x for x in exports if x[0]<=offset]
        if candidates:
            address,name=candidates[-1];symbol=dict(name=name,export_rva=address,offset=offset-address,kind='nearest export; function extent unavailable')
        return dict(ip=ip,module=m['path'],module_sha256=m.get('sha256'),loaded_base=m['base'],offset=offset,symbol=symbol)
    def finish(self,outcome):
        self.finished=True;self.active=False
        for b in self.pending.values():
            if b:self.counts['incomplete_lines']+=1;self.counts['excluded_bytes']+=len(b)
        self.retain_terminal(outcome.get('fault_status'));self.resolve_modules()
        exceptions=[]
        retained=list(self.exceptions)
        for e in self.fatal_reserved:
            if e not in retained:retained.append(e)
        for old in retained:
            e=dict(old);e['fault']=self.frame(e['windows_pid'],e['ip'],at=e['timestamp']);e['stack']=[self.frame(e['windows_pid'],x['ip'],x['base'],at=e['timestamp']) for x in e.pop('frames')]
            exit_row=self.exits.get(e['windows_pid']);e['classification']='observed_exception_fatality_unavailable'
            if exit_row and exit_row['exit_code']==0:e['classification']='exception_followed_by_normal_exit'
            elif exit_row and exit_row['exit_code']==e['code'] and e['code']>=0x80000000:e['classification']='exception_matching_self_exit_status'
            elif e['unhandled_marker']:e['classification']='unhandled_marker_observed_exit_correlation_unavailable'
            e['belongs_to_selected_windows_host']=e['windows_pid'] in self.host_pids;exceptions.append(e)
        native_now=self.identity(self.native['pid']) if self.native else None
        native_same=bool(self.native and native_now and native_now['start_ticks']==self.native['start_ticks'])
        linux_hosts=[r for r in self.processes.values() if any(self.reg['host']['path'] in (x['path'],x['resolved_path']) for x in r.get('mapped_files',[]))]
        capture=dict(schema=1,incident=self.request['id'],session=self.sid,request=self.request,
            state='cancelled_capture' if (self.path/'cancel.json').exists() else 'finalized',
            retention=dict(stop_reason=self.stop_reason,elapsed_seconds=time.monotonic()-self.started),
            processes=dict(proton_launcher=dict(identity=self.launcher,exit_before_cleanup=outcome.get('exit_before_cleanup'),exit_after_cleanup=outcome.get('raw_exit')),
                windows_host=dict(windows_pids=sorted(self.host_pids),linux_identities=linux_hosts,exit_observations=[v for k,v in self.exits.items() if k in self.host_pids],unix_wait_status=None,unix_wait_status_unavailable='Windows child is not the supervisor Popen child'),
                native_host=dict(identity=self.native,same_identity_at_finalization=native_same,exit_status=None,exit_status_unavailable='not a child of this supervisor; final native retirement may occur later')),
            modules=self.modules,exceptions=exceptions,first_terminal=self.first_terminal,
            fault_status=outcome.get('fault_status'),cleanup={k:outcome.get(k) for k in ['cleanup_confirmed','transport_retired','retirement_disposition','vendor_retirement','error']},
            recent_context=self.context.value(),terminal_context=self.terminal.value(),
            limits=dict(line_bytes=self.LINE,modules=self.MODULES,exceptions=self.EXCEPTIONS,frames_per_exception=self.FRAMES,processes=self.PROCESSES,retention_seconds=self.SECONDS,module_resolution_seconds=self.FINAL_SECONDS,mapped_file_record_bytes=524288,context=self.context.limits(),terminal=self.terminal.limits()),counts=self.counts,
            interpretation='Exception/exit correlation is evidence of a failure path, not proof that the faulting module owns the underlying defect. Missing symbols and unavailable statuses remain explicit.',reporting_error=self.error)
        data=json.dumps(capture).encode()
        if len(data)>4*1024*1024:raise ValueError('incident JSON capacity')
        atomic(self.path/'incident.json',capture)
        lines=[f"Incident {self.request['id']}",f"Session {self.sid}",f"Outer Proton exit: {outcome.get('raw_exit')}; Windows status is recorded separately.",f"IF1 terminal: {self.first_terminal}",f"Cleanup: {capture['cleanup']}"]
        for e in [x for x in exceptions if x['belongs_to_selected_windows_host']][-4:]:
            lines.append(f"Windows process {e['windows_pid']:x}, thread {e['windows_tid']:x}: exception 0x{e['code']:08x}; {e['classification']}")
            for frame in [e['fault'],*e['stack'][:8]]:lines.append(f"  {frame.get('module')} + {hex(frame['offset']) if frame.get('offset') is not None else 'unavailable'}; {frame.get('symbol')}")
        if not any(e['belongs_to_selected_windows_host'] for e in exceptions):lines.append('No attributable Windows exception stack retained. SIGKILL/transport loss alone cannot supply one.')
        lines.append(capture['interpretation']);lines.append('Recent allowlisted error context (private; a lead, not cause):');lines.extend(self.context.value()[-8:]);lines.append('Drops/incompleteness: '+json.dumps(self.counts))
        text='';summary_dropped=0
        for line in lines:
            if len(text.encode())+len(line.encode())+1>65000:summary_dropped+=1;continue
            text+=line+'\n'
        text+=f'Summary lines omitted by capacity: {summary_dropped}; full bounded fields remain in incident.json.\n'
        fd=os.open(self.path/'summary.txt',os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW,0o600)
        with os.fdopen(fd,'w') as f:f.write(text)
        share=dict(schema=1,outcome=capture['cleanup']['retirement_disposition'],cleanup_confirmed=bool(outcome.get('cleanup_confirmed')),transport_retired=bool(outcome.get('transport_retired')),outer_exit=outcome.get('raw_exit'),windows_self_exit_codes=[v['exit_code'] for k,v in self.exits.items() if k in self.host_pids],native_same_identity_at_finalization=native_same,first_terminal={k:self.first_terminal[k] for k in ('generation','epoch','sequence','last_completed_position','state_revision','state_sha256','failure_class','status','producer','status_domain') if k in self.first_terminal} if self.first_terminal else None,exceptions=[],counts=self.counts)
        for e in exceptions:
            if e['belongs_to_selected_windows_host']:
                share['exceptions'].append(dict(code=e['code'],classification=e['classification'],fault={k:e['fault'].get(k) for k in ['module_sha256','offset']},stack=[{k:x.get(k) for k in ['module_sha256','offset']} for x in e['stack']]))
        atomic(self.path/'share.json',share);self.status(capture['state']);return capture

def run(spec,peer=None):
    os.umask(0o077);reg=spec['registration'];directory,durable=session_directories(spec);sid=spec['session'];report=pathlib.Path(spec['report'])
    for item in [reg['host'],reg['module'],*reg['environment']['runner']['files']]:verify(item)
    cmd,binding=command(spec);env=environment(reg)
    managed_home(spec,env)
    transport_environment(spec,env);delivery_trace(spec,env);stop=False
    capture=None;capture_error=None
    if spec.get('crash_capture'):
        try:
            capture=IncidentCapture(spec,peer)
            env=vendor_diagnostic_environment(env,capture.path,True)
        except Exception as e:capture_error=type(e).__name__+': '+str(e)[:256]
    def capture_call(method,*args):
        nonlocal capture_error
        if capture is None:return
        try:return getattr(capture,method)(*args)
        except Exception as e:
            capture_error=type(e).__name__+': '+str(e)[:256]
            capture.error=capture_error
            if method!='finish':capture.active=False
            return None
    def stopped(*_):
        nonlocal stop
        stop=True
    signal.signal(signal.SIGTERM,stopped);signal.signal(signal.SIGINT,stopped)
    disconnected=None
    def native_released():
        nonlocal disconnected
        if peer is None:return False
        if disconnected is None:
            try:
                data=peer.recv(1)
                if data:raise RuntimeError('unexpected native owner bytes')
                disconnected=time.monotonic()
            except BlockingIOError:pass
            except OSError:disconnected=time.monotonic()
        return disconnected is not None
    def native_stopped():
        return stop or (native_released() and time.monotonic()-disconnected>=2)
    if peer is not None:
        if not spec.get('binding_sent'):
            reply=(sid+'\n'+str(directory)).encode();peer.settimeout(5);peer.sendall(struct.pack('<H',len(reply))+reply)
        peer.setblocking(False)
        end=time.monotonic()+10
        while not (directory/'ap1.control').exists():
            if native_stopped() or time.monotonic()>=end:raise RuntimeError('native setup disconnected or timed out')
            time.sleep(.02)
    if not spec['inspect'] and not spec.get('vendor_access'):ResultStatus.create(directory,sid)
    visibility=FaultStatus(directory,sid) if not spec['inspect'] and not spec.get('vendor_access') else None
    retirement=None;retirement_ready=None
    if spec['inspect'] or spec.get('vendor_access'):
        env.pop('LVB_VENDOR_RETIREMENT',None) # companion/inspection ownership is distinct
    elif env.get('LVB_VENDOR_RETIREMENT'):
        RetirementStatus.create(directory,sid);retirement=RetirementStatus(directory,sid)
    root=subprocess.Popen(cmd,env=env,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True,bufsize=0)
    records=[];owned=set();pending=bytearray();vendor=bytearray();stderr=bytearray();dropped={'vendor':0,'stderr':0};protocol_bytes=0;gated=False;call=None;started=time.monotonic();failure=None;clean=False;code=None
    sel=selectors.DefaultSelector()
    def retain(key,target,data):
        n=min(len(data),max(0,65536-len(target)));target.extend(data[:n]);dropped[key]+=len(data)-n
    def feed(data):
        nonlocal protocol_bytes,call,gated
        pending.extend(data)
        while b'\n' in pending:
            line,_,rest=pending.partition(b'\n');pending[:]=rest
            if not line.startswith(b'{"event":'):
                retain('vendor',vendor,line+b'\n');capture_call('write','vendor',line+b'\n');continue
            protocol_bytes+=len(line)+1
            if protocol_bytes>1048576:raise RuntimeError('host protocol output capacity exceeded')
            record=json.loads(line);records.append(record);state=record.get('state')
            if state=='ap8_call' or record.get('event')=='call_started':call=(record.get('operation'),time.monotonic())
            elif state in ('ap8_result','ap8_failure','ap8_inspection_closed') or record.get('event')=='call_completed':call=None
        if len(pending)>65536:raise RuntimeError('host output line capacity exceeded')
    def pump(timeout):
        for key,_ in sel.select(timeout):
            data=os.read(key.fileobj.fileno(),16384)
            if not data:sel.unregister(key.fileobj);continue
            if key.data=='stdout':feed(data)
            else:
                retain('stderr',stderr,data);capture_call('write','stderr',data)
    try:
        for stream,label in [(root.stdout,'stdout'),(root.stderr,'stderr')]:os.set_blocking(stream.fileno(),False);sel.register(stream,selectors.EVENT_READ,label)
        tracker=ProcessTracker(root.pid)
        while True:
            owned.update(tracker.update())
            capture_call('observe',owned,root)
            pump(.05)
            if visibility:
                was=visibility.suspect
                visibility.poll()
                if was is None and visibility.suspect is not None:
                    if env.get('LVB_AP10_TRACE')=='1':visibility.suspect['threads']=fault_threads(owned)
                    try:atomic(report.with_suffix('.fault.json'),{'session':sid,'early_pending':visibility.suspect})
                    except OSError:pass # final outcome also retains this bounded witness
            if retirement:
                ready=retirement.snapshot()
                if ready is not None:
                    # Outer exit before the supervisor accepted custody remains
                    # abnormal; a committed row does not reclassify a crash.
                    if root.poll() is not None:raise RuntimeError('Windows exited before process retirement containment')
                    retirement_ready=ready;break
            if native_stopped():
                if retirement:failure='Windows retirement status absent'
                if spec.get('vendor_access'):
                    (directory/'vendor.stop').write_text(sid+'\n')
                    end=time.monotonic()+10
                    while time.monotonic()<end and not any(r.get('state')=='scanner_completed' for r in records):pump(.05)
                break
            if not gated and any(r.get('state')=='readiness_announced' for r in records):
                ready=durable/(sid+'.ready');m=ready.lstat()
                if not stat.S_ISREG(m.st_mode) or m.st_size>1024 or ready.read_bytes()!=binding:raise RuntimeError('Windows readiness binding differs')
                for item in [reg['host'],reg['module']]:verify(item)
                windows_transport_views(spec)
                gate=durable/(sid+'.gate');temp=durable/(sid+'.gate.tmp')
                with temp.open('xb') as f:f.write(binding);f.flush();os.fsync(f.fileno())
                temp.replace(gate);gated=True
            now=time.monotonic()
            if not gated and now-started>180:raise TimeoutError('Windows startup deadline')
            if call and now-call[1]>30:raise TimeoutError('Windows call deadline: '+str(call[0]))
            if spec['inspect'] and now-started>180:raise TimeoutError('inspection deadline')
            # A companion can keep the outer Proton launcher alive after the SDK
            # host has finished. Its terminal result is authoritative for that
            # host; the launcher return code is retained separately, never invented.
            terminal=next((r for r in reversed(records) if r.get('state')=='ap8_inspection_closed'),None)
            if terminal and terminal.get('exit_code'):
                if visibility:visibility.terminal.root_exit(terminal['exit_code'],3)
                failure='Windows SDK host failed: '+str(terminal['exit_code']);break
            if any(r.get('state')=='scanner_completed' for r in records):break
            if root.poll() is not None:
                code=root.returncode
                if visibility and retirement_ready is None:visibility.terminal.root_exit(code)
                for _ in range(20):
                    if not sel.get_map():break
                    pump(.01)
                if code or not any(r.get('state')=='scanner_completed' for r in records):failure='Windows host exited without successful close'
                break
    except Exception as e:failure=f'{type(e).__name__}: {e}'
    finally:
        if retirement:
            retirement.close()
            if retirement_ready is None and failure is None:failure='Windows retirement status absent'
        fault=None;fault_reporting_error=None
        if visibility:
            try:
                if retirement_ready is None and root.poll() is not None:visibility.terminal.root_exit(root.returncode)
                fault={'session':sid,'early_pending':visibility.suspect,'before_containment':visibility.snapshot()}
                atomic(report.with_suffix('.fault.json'),fault)
                capture_call('retain_terminal',fault)
            except Exception as e:fault_reporting_error=type(e).__name__+': '+str(e)[:256]
            finally:visibility.close()
        try:
            def cleanup_drain():
                nonlocal failure
                try:pump(0)
                except Exception as e:failure=failure or ('cleanup diagnostic drain: '+type(e).__name__)
            cleanup=cleanup_process(root,sorted(owned),during_cleanup=cleanup_drain) if capture else cleanup_process(root,sorted(owned))
            clean=all(cleanup.values())
            if capture:
                # The process owner is already retired. A closed or saturated
                # logging stream cannot introduce an unbounded final wait.
                end=time.monotonic()+.25
                while sel.get_map() and time.monotonic()<end:cleanup_drain()
                capture.counts['final_drain_incomplete']=bool(sel.get_map())
        except Exception as e:
            failure=(failure+'; ' if failure else '')+'cleanup: '+str(e)
        sel.close()
        for stream in (root.stdout,root.stderr):stream.close()
        outcome={'vendor_retirement':retirement_ready,'transport_storage':spec.get('transport'),'fault_status':fault,'fault_reporting_error':fault_reporting_error,'ownership_schema':1,'session':sid,'records':records,'exit_before_cleanup':code,'raw_exit':root.returncode,'error':failure,'cleanup_confirmed':clean,'gated':gated,'discarded_diagnostic_bytes':dropped,'vendor_stdout':vendor.decode(errors='replace'),'stderr':stderr.decode(errors='replace')}
        # Diagnostic persistence cannot skip physical cleanup or peer retirement.
        try:atomic(report,outcome)
        except OSError as e:outcome['reporting_error']=type(e).__name__+': '+str(e)[:256]
    if clean:
        retired=peer is None or disconnected is not None
        if peer is not None and not retired:
            # Wake the native transport accept/worker on early Windows failure.
            # Never acknowledge retirement until the native owner releases it.
            try:
                if failure:peer.sendall(b'F')
                end=time.monotonic()+10
                while time.monotonic()<end:
                    if native_released():retired=True;break
                    time.sleep(.02)
            except OSError:pass
        outcome['transport_retired']=retired
        if retired:
            # Only this random, private session is removed. Reports live outside
            # it; no environment, vendor, publication or sibling path is touched.
            try:retire_directories(spec)
            except (OSError,RuntimeError) as e:
                outcome['transport_retired']=False
                outcome['retirement_error']=type(e).__name__+': '+str(e)[:256]
            if outcome['transport_retired'] and peer is not None:
                try:peer.settimeout(5);peer.sendall(b'R')
                except OSError:pass
        try:atomic(report,outcome)
        except OSError as e:outcome['reporting_error']=type(e).__name__+': '+str(e)[:256]
    if retirement_ready is not None and clean and outcome.get('transport_retired') and not failure:
        outcome['retirement_disposition']='process_scoped_vendor_retirement'
        try:atomic(report,outcome)
        except OSError as e:outcome['reporting_error']=type(e).__name__+': '+str(e)[:256]
    if fault and fault.get('before_containment',{}).get('terminal_instance'):
        outcome['retirement_disposition']='terminal_instance_failure'
        try:atomic(report,outcome)
        except OSError:pass
    # Minimal ownership receipt is independent of the rich report, with a
    # bounded stdout result to the live Rust parent even if persistence fails.
    receipt={k:outcome.get(k,False) for k in ('session','cleanup_confirmed','transport_retired')}
    receipt['reporting_error']=outcome.get('reporting_error')
    try:atomic(report.with_suffix('.ownership.json'),receipt)
    except OSError as e:outcome['ownership_reporting_error']=type(e).__name__+': '+str(e)[:256]
    if capture:
        capture_call('finish',outcome)
    if spec.get('crash_capture'):
        outcome['incident']={'id':spec['crash_capture'].get('id'),'reporting_error':capture_error}
        if capture_error:
            try:
                atomic(pathlib.Path(spec['crash_capture']['directory'])/'status.json',{'state':'reporting_failed','capture_enabled':False,'error':capture_error,'cleanup_confirmed':clean,'transport_retired':outcome.get('transport_retired')})
            except OSError:pass
        try:atomic(report,outcome)
        except OSError:pass
    return outcome

def keep(spec):
    """Own the shared Wine infrastructure before any DSP instance is admitted.

    The Windows host waits on a private lifetime marker. It loads no plug-in.
    This keeps the wineserver/services out of either audio instance's subtree.
    """
    reg=spec['registration'];runner=reg['environment']['runner'];report=pathlib.Path(spec['report']);stop=False
    def stopped(*_):
        nonlocal stop
        stop=True
    signal.signal(signal.SIGTERM,stopped);signal.signal(signal.SIGINT,stopped)
    directory=pathlib.Path(spec['directory']);verify(reg['host'])
    cmd=[runner['entry_point'],'--verb=run','--',runner['proton'],'runinprefix',windows(reg['host']['path'],pathlib.Path(reg['environment']['root'])/'compatdata/pfx'),'--environment-owner',spec['session'],'--scanner-sha256',reg['host']['sha256']]
    env=environment({**reg,'compatibility':{'disable_windows_accessibility':False}});managed_home(spec,env);transport_environment(spec,env)
    root=subprocess.Popen(cmd,env=env,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True,bufsize=0)
    sel=selectors.DefaultSelector();owned=set();text=bytearray();ready=False;started=time.monotonic();error=None;clean=False
    for pipe in (root.stdout,root.stderr):os.set_blocking(pipe.fileno(),False);sel.register(pipe,selectors.EVENT_READ)
    try:
        tracker=ProcessTracker(root.pid)
        while not stop:
            owned.update(tracker.update())
            for key,_ in sel.select(.05):
                data=os.read(key.fileobj.fileno(),4096)
                if not data:sel.unregister(key.fileobj)
                elif key.fileobj==root.stdout:text.extend(data[:max(0,65536-len(text))])
            if not ready and (directory/'environment.ready').exists():
                if (directory/'environment.ready').read_bytes()!=(spec['session']+'\n').encode():raise RuntimeError('environment readiness binding differs')
                atomic(report,{'ready':True,'environment':reg['environment']['id']});ready=True
            if root.poll() is not None:raise RuntimeError('shared environment owner exited')
            if not ready and time.monotonic()-started>180:raise TimeoutError('environment startup deadline')
    except Exception as e:error=str(e)
    finally:
        root.stdin.close()
        (directory/'environment.stop').write_text(spec['session']+'\n')
        try:root.wait(timeout=2)
        except subprocess.TimeoutExpired:pass
        try:clean=all(cleanup_process(root,sorted(owned)).values())
        except Exception as e:error=(error+'; ' if error else '')+str(e)
        sel.close();root.stdout.close();root.stderr.close()
        atomic(report,{'ready':False,'cleanup_confirmed':clean,'error':error,'raw_exit':root.returncode})
    return {'cleanup_confirmed':clean}

def install(spec):
    if spec.get('schema') in (2,3):return managed_install(spec)
    if 'installer_capability' in spec:raise RuntimeError('installer policy requires schema 3')
    env=spec['environment'];rootdir=pathlib.Path(env['root']);lock=(rootdir/'operation.lock').open('a+b');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    for a in [spec['installer'],*env['runner']['files']]:verify(a)
    reg={'environment':env,'compatibility':{'disable_windows_accessibility':False}};runner=env['runner'];command=[runner['entry_point'],'--verb=run','--',runner['proton'],'run',spec['installer']['path']]
    child=subprocess.Popen(command,env=environment(reg),stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True,bufsize=0)
    owned=set();sel=selectors.DefaultSelector();log=bytearray();discarded=0;started=time.monotonic();failure=None;clean=False;code=None
    for pipe in (child.stdout,child.stderr):os.set_blocking(pipe.fileno(),False);sel.register(pipe,selectors.EVENT_READ)
    try:
        while True:
            census=process_identities();owned.update((r['pid'],r['start_ticks']) for r in census if r['pid']==child.pid);owned.update((r['pid'],r['start_ticks']) for r in descendant_identities(child.pid,census))
            for key,_ in sel.select(.05):
                data=os.read(key.fileobj.fileno(),16384)
                if not data:sel.unregister(key.fileobj)
                n=min(len(data),max(0,65536-len(log)));log.extend(data[:n]);discarded+=len(data)-n
            if child.poll() is not None:code=child.returncode;break
            if time.monotonic()-started>3600:raise TimeoutError('installer exceeded one-hour bound')
    except Exception as e:failure=str(e)
    finally:
        try:clean=all(cleanup_process(child,sorted(owned)).values())
        except Exception as e:failure=(failure+'; ' if failure else '')+str(e)
        sel.close();child.stdout.close();child.stderr.close()
        atomic(pathlib.Path(spec['report']),{'installer':spec['installer'],'raw_exit':code,'cleanup_confirmed':clean,'error':failure,'discarded_diagnostic_bytes':discarded,'private_log':log.decode(errors='replace')})
        lock.close()
    return code==0 and clean and failure is None

def installer_reap(child):
    if child:child.poll()
    for _ in range(128):
        try:
            pid,status=os.waitpid(-1,os.WNOHANG)
            if not pid:break
            # The launcher can exit between poll and waitpid. Preserve its
            # actual status rather than letting Popen infer zero after ECHILD.
            if child is not None and pid==child.pid:child.returncode=os.waitstatus_to_exitcode(status)
        except ChildProcessError:break


class InstallerStartup:
    """Bounded installer-only observations; a cancelled cohort does not erase a fault.

    Output is private. Only closed diagnostic signatures enter the typed report;
    arbitrary stderr text stays in the existing bounded private sink.
    """
    def __init__(self, op, artifact, root, clock=time.monotonic_ns):
        self.op=op;self.clock=clock;self.begin=clock();self.rows=[];self.dropped=0
        self.first_problem=None;self.cancel=None;self.pending={};self.discarding=set();self.line_drops=0
        self.artifact=artifact;self.root=pathlib.Path(root);self.target=None;self.helpers=[];self.next_census=0;self.images={}
        self.stage('supervisor_started')
    def stage(self, kind, **fields):
        row={'stage':kind,'elapsed_ns':self.clock()-self.begin,**fields}
        if len(self.rows)<48:self.rows.append(row)
        else:self.dropped+=1
    def problem(self, code, status=None, source="supervisor"):
        if self.first_problem is None:
            self.first_problem={'code':code,'status':status,'elapsed_ns':self.clock()-self.begin,'source':source}
            self.stage('startup_problem_observed',code=code)
    def feed(self, data, stream="stderr"):
        # Bounded line assembly; oversized/truncated text cannot become a guessed
        # exception or path. Keep draining even after the retention bound.
        if stream not in ('stdout','stderr'):raise ValueError('diagnostic stream')
        for line in (self.pending.get(stream,b'')+data).splitlines(keepends=True):
            complete=line.endswith((b'\n',b'\r'))
            if stream in self.discarding:
                if complete:self.discarding.remove(stream)
                continue
            if len(line)>4096:
                self.line_drops+=1;self.pending[stream]=b''
                if not complete:self.discarding.add(stream)
                continue
            if not line.endswith((b'\n',b'\r')):self.pending[stream]=line;continue
            self.pending[stream]=b''
            if b'Xalia' in line:continue
            if b'err:steamclient:' in line and b'unable to load native steamclient library' in line:
                self.problem('native_steamclient_load_failed',source='bounded_runner_pipe_signature')
            elif b'err:steamclient:' in line and b'unable to load ' in line:
                self.problem('native_steamclient_export_unavailable',source='bounded_runner_pipe_signature')
            elif b'Assertion failed' in line or b'Assertion ' in line and b' failed' in line:
                self.problem('runtime_assertion_observed',source='bounded_runner_pipe_signature')
    def cancellation(self):
        if self.cancel is None:
            self.cancel={'elapsed_ns':self.clock()-self.begin,'source':'owned_unit_stop_signal'}
            self.stage('cancellation_requested')
    def bind_images(self, runner):
        candidates=[('target',pathlib.Path(self.artifact['path']))]
        base=pathlib.Path(runner['proton']).parent/'files/lib/wine/x86_64-windows'
        candidates += [(name,base/name) for name in ('steam.exe','wineboot.exe')]
        for name,path in candidates:
            try:
                with os.fdopen(os.open(path,os.O_RDONLY|os.O_NOFOLLOW),'rb') as f:
                    md=os.fstat(f.fileno())
                    if not stat.S_ISREG(md.st_mode):continue
                    # The target is already verified by the launch owner; helpers
                    # have a small explicit digest bound and no public pathname.
                    if name=='target':digest=self.artifact['sha256']
                    elif md.st_size<=16*1024*1024:digest=hashlib.file_digest(f,'sha256').hexdigest()
                    else:continue
                    self.images[(os.major(md.st_dev),os.minor(md.st_dev),md.st_ino)]=(name,digest)
            except OSError:continue
    def observe(self, members):
        now=self.clock()
        if now<self.next_census:return
        self.next_census=now+100_000_000
        # Identity is the mapped file's device/inode, never comm or basename.
        for member in members[:64]:
            try:
                pid=member['pid'];proc=pathlib.Path('/proc')/str(pid)
                with (proc/'maps').open('rb') as f:raw=f.read(524289)
                observed_start=int((proc/'stat').read_text().rsplit(')',1)[1].split()[19])
                if observed_start!=member['start_ticks'] or len(raw)>524288:continue
                for line in raw.decode(errors='replace').splitlines():
                    parts=line.split(None,5)
                    if len(parts)!=6:continue
                    dev=tuple(int(x,16) for x in parts[3].split(':'));ino=int(parts[4])
                    image=self.images.get((*dev,ino))
                    if image is None:continue
                    name,digest=image
                    row={'pid':pid,'start_ticks':member['start_ticks'],'image':name,'sha256':digest}
                    if name=='target':
                        if self.target is None:self.target=row;self.stage('target_image_observed')
                    elif len(self.helpers)<8 and row not in self.helpers:
                        self.helpers.append(row);self.stage('helper_image_observed',image=name)
            except (OSError,ValueError,KeyError):continue
    def value(self):
        return {'schema':1,'operation':self.op,'stages':self.rows,'dropped_stages':self.dropped,
                'first_problem':self.first_problem,'cancellation':self.cancel,'target':self.target,
                'target_observation':'observed' if self.target else 'unknown','target_ready':'unknown',
                'helpers':self.helpers,'diagnostic_line_drops':self.line_drops,
                'exception_stack':'unavailable'}

# IS1 installer-only observations. Private records are separate from the small
# manager receipt and from every validated VST/host protocol.
def installer_atomic(path, value):
    """Private replace, including recovery from an interrupted prior write."""
    import tempfile
    fd,name=tempfile.mkstemp(prefix='.'+path.name+'-',dir=path.parent)
    try:
        with os.fdopen(fd,'w') as f:json.dump(value,f,separators=(',',':'));f.flush();os.fsync(f.fileno())
        os.replace(name,path)
        d=os.open(path.parent,os.O_RDONLY|os.O_DIRECTORY)
        try:os.fsync(d)
        finally:os.close(d)
    finally:
        try:os.unlink(name)
        except FileNotFoundError:pass

def installer_open(path):
    # Every component is no-follow, not just the leaf. An installer-created
    # junction/symlink cannot turn observation into an operator-HOME export.
    path=pathlib.Path(path)
    if not path.is_absolute() or '..' in path.parts:raise ValueError('installer observation location')
    fd=os.open('/',os.O_RDONLY|os.O_DIRECTORY)
    try:
        for part in path.parts[1:-1]:
            next_fd=os.open(part,os.O_RDONLY|os.O_DIRECTORY|os.O_NOFOLLOW,dir_fd=fd)
            os.close(fd);fd=next_fd
        result=os.open(path.name,os.O_RDONLY|os.O_NOFOLLOW|os.O_NONBLOCK,dir_fd=fd)
        if not stat.S_ISREG(os.fstat(result).st_mode):
            os.close(result);raise ValueError('installer observation requires regular file')
        return result
    finally:os.close(fd)

def installer_digest(path, maximum=64*1024*1024):
    with os.fdopen(installer_open(path),'rb') as f:
        a=os.fstat(f.fileno())
        if not stat.S_ISREG(a.st_mode) or a.st_size>maximum:raise ValueError('installer image bound/type')
        header=f.read(64);kind='unknown';pe=None
        if header[:2]==b'MZ' and len(header)==64:
            offset=struct.unpack_from('<I',header,60)[0]
            if offset<=min(a.st_size-24,262144):
                f.seek(offset);coff=f.read(24)
                if coff[:4]==b'PE\0\0':
                    kind='pe_executable';pe={'machine':struct.unpack_from('<H',coff,4)[0],'characteristics':struct.unpack_from('<H',coff,22)[0]}
        elif header[:8]==bytes.fromhex('d0cf11e0a1b11ae1'):kind='compound_file'
        f.seek(0);digest=hashlib.file_digest(f,'sha256').hexdigest();b=os.fstat(f.fileno())
        stamp=lambda m:(m.st_dev,m.st_ino,m.st_size,m.st_mtime_ns,m.st_ctime_ns)
        if stamp(a)!=stamp(b):raise ValueError('installer image changed')
        return {'sha256':digest,'size':a.st_size,'device':a.st_dev,'inode':a.st_ino,'format':kind,'pe':pe}

def installer_walk(root,incomplete,deadline,budget):
    # os.walk materializes an entire directory before a caller can enforce a
    # bound. Use a capped iterator instead; no filesystem-wide enumeration.
    pending=[root]
    while pending:
        directory=pending.pop();files=[]
        try:
            if directory.resolve()!=directory:incomplete.append('aliased_directory');continue
            with os.scandir(directory) as entries:
                for entry in entries:
                    budget[0]-=1
                    if budget[0]<0 or time.monotonic()>deadline:
                        incomplete.append('file_count_or_time_bound');return
                    if entry.is_dir(follow_symlinks=False):pending.append(pathlib.Path(entry.path))
                    elif entry.is_file(follow_symlinks=False):files.append(entry.name)
            yield directory,[],files
        except FileNotFoundError:continue
        except OSError:incomplete.append('directory_unavailable')

class InstallerWitnesses:
    """Allowlisted installation metadata, not a prefix/account-data export.

    Paths stay private. A registration is not proof of authorization, dependency
    health or a successful post-install launch. Bounds make incomplete explicit.
    """
    def __init__(self, root):self.prefix=pathlib.Path(root)/'compatdata/pfx'
    def snapshot(self):
        out={'files':{},'logs':{},'uninstall':{},'services':{},'incomplete':[]}
        c=self.prefix/'drive_c';count=0;budget=256*1024*1024;deadline=time.monotonic()+5;walk_budget=[8192]
        roots=[c/'Program Files',c/'Program Files (x86)',c/'ProgramData',c/'windows/temp']
        users=c/'users'
        if users.is_dir():
            for user in sorted(users.iterdir())[:16]:
                if not user.is_symlink():roots.append(user/'Temp');roots.append(user/'AppData/Local/Temp')
        for root in roots:
            if root.resolve()!=root:out['incomplete'].append('aliased_root');continue
            for directory,dirs,files in installer_walk(root,out["incomplete"],deadline,walk_budget):
                dirs[:]=sorted(d for d in dirs if not (pathlib.Path(directory)/d).is_symlink())
                count+=len(files)+1
                if count>8192 or time.monotonic()>deadline:out['incomplete'].append('file_count_or_time_bound');break
                for name in sorted(files):
                    p=pathlib.Path(directory)/name;suffix=p.suffix.lower()
                    if suffix not in ('.exe','.dll','.msi','.log'):continue
                    try:
                        m=p.lstat()
                        if not stat.S_ISREG(m.st_mode):continue
                        relative=str(p.relative_to(c))
                        if suffix=='.log':
                            # No content here; new/changed logs may be retained by
                            # a bounded private-tail collector after finalization.
                            out['logs'][relative]={'size':m.st_size,'mtime_ns':m.st_mtime_ns};continue
                        row={'size':m.st_size,'sha256':None}
                        if m.st_size<=min(budget,64*1024*1024):
                            budget-=m.st_size;row.update(installer_digest(p))
                        else:out['incomplete'].append('image_hash_bound')
                        out['files'][relative]=row
                    except (OSError,ValueError):out['incomplete'].append('file_unavailable_or_changed')
            if count>8192 or time.monotonic()>deadline:break
        for registry in ('system.reg','user.reg'):
            path=self.prefix/registry
            try:
                with os.fdopen(installer_open(path),'rb') as f:data=f.read(8*1024*1024+1)
                if len(data)>8*1024*1024:out['incomplete'].append('registry_bound');continue
            except (OSError,ValueError):out['incomplete'].append('registry_unavailable');continue
            kind=None;key=None
            for line in data.decode(errors='replace').splitlines():
                if line.startswith('['):
                    raw=line[1:line.find(']')].replace('\\\\','\\');low=raw.lower();kind=None;key=None
                    if '\\uninstall\\' in low:kind='uninstall'
                    elif re.search(r'\\controlset\d{3}\\services\\[^\\]+$',low):kind='services'
                    if kind:
                        key=hashlib.sha256((registry+':'+raw).encode()).hexdigest();out[kind].setdefault(key,{})
                elif kind and line.startswith('"'):
                    match=re.fullmatch(r'"(DisplayName|DisplayVersion|InstallLocation|ImagePath|Start|Type)"=(.*)',line)
                    if match:
                        name,value=match.groups()
                        # ImagePath can contain command arguments/secrets. Retain
                        # only a digest and the executable location, never args.
                        if name=='ImagePath':
                            out[kind][key]['image_configuration_sha256']=hashlib.sha256(value.encode()).hexdigest()
                            decoded=value.strip('"').replace('\\\\','\\').replace('\\"','"')
                            image=re.match(r'^"?([A-Za-z]:\\.*?\.exe)(?:"|\s|$)',decoded,re.I)
                            if image:out[kind][key]['image']=image.group(1)
                        elif name in ('InstallLocation','DisplayName','DisplayVersion'):
                            if len(value)<=1024:out[kind][key][name]=value.strip('"').replace('\\\\','\\')
                        elif re.fullmatch('dword:[0-9a-fA-F]{8}',value):out[kind][key][name]=value
        out['incomplete']=sorted(set(out['incomplete']));return out

    @staticmethod
    def compare(before,after):
        delta={}
        for key in ('files','logs','uninstall','services'):
            a=before.get(key,{});b=after.get(key,{})
            delta[key]={'added':sorted(k for k in b if k not in a),'changed':sorted(k for k in b if k in a and b[k]!=a[k]),'removed':sorted(k for k in a if k not in b)}
        registrations=[]
        for key in delta['uninstall']['added']+delta['uninstall']['changed']:
            location=after['uninstall'][key].get('InstallLocation','').replace('\\','/').rstrip('/')
            if not location.lower().startswith('c:/'):continue
            relative=location[3:].lower()+'/'
            images=[p for p in delta['files']['added']+delta['files']['changed'] if p.lower().startswith(relative) and p.lower().endswith('.exe') and after['files'][p].get('sha256') and after['files'][p].get('format')=='pe_executable']
            if images:registrations.append({'registration':key,'images':images})
        changed=any(delta[k][change] for k in ('files','uninstall','services') for change in ('added','changed','removed'))
        return {'delta':delta,'application_registrations':registrations,
            'classification':'installed' if registrations else 'partial_installation' if changed else 'indeterminate' if before.get('incomplete') or after.get('incomplete') else 'not_installed',
            'dependency_health':'unproved','postinstall_launch':'unproved',
            'incomplete':sorted(set(before.get('incomplete',[])+after.get('incomplete',[])))}

class InstallerPresence:
    """Closed script-operation observations, not interpreter or match-result authority.

    Commands remain only in the existing bounded private runner sink. Their exact
    UTF-16 digest and a strict grammar classify a request, never its execution.
    A helper exit zero cannot establish presence, successful close or recheck.
    """
    def __init__(self):self.rows=[];self.dropped=0
    @staticmethod
    def request(body):
        m=re.search(r' cmdline L"((?:[^"\\]|\\.)*)", inherit ',body)
        if not m:return None
        try:command=json.loads('"'+m[1]+'"')
        except (ValueError,UnicodeError):return None
        m=re.fullmatch(r'"([^"\r\n]+)" -C "([^"\r\n]{1,3072})"',command)
        if not m or m[1].replace('\\','/').rsplit('/',1)[-1].lower()!='powershell.exe':return None
        script=m[2];kind='unknown';predicate=None
        query=r"Get-CimInstance -ClassName Win32_Process \| \? \{\$_\.Path -and \$_\.Path\.StartsWith\('([^'\r\n]+)', 'CurrentCultureIgnoreCase'\)\}"
        found=re.fullmatch(r'if \(\('+query+r'\)\.Count -gt 0\) \{ exit 0 \} else \{ exit 1 \}',script)
        if found:kind='presence_query';predicate=found[1]
        else:
            found=re.fullmatch(query+r' \| % \{ Stop-Process -Id \$_\.ProcessId( -Force)? \}',script)
            if found:kind='close_request';predicate=found[1]
            elif script=='if (Get-Command Get-CimInstance -ErrorAction SilentlyContinue) { exit 0 } else { exit 1 }':kind='capability_query'
        return {'mechanism':'powershell_wmi','operation_class':kind,'query_sha256':hashlib.sha256(script.encode('utf-16le')).hexdigest(),
            'predicate_sha256':hashlib.sha256(predicate.encode('utf-16le')).hexdigest() if predicate else None,
            'scope':'executable_path_prefix' if predicate else 'unavailable','matched_object_count':None,'matched_objects':'unavailable_no_query_result_observation',
            'close_mechanism':'script_process_api' if kind=='close_request' else None,'close_result':'unavailable' if kind=='close_request' else None,
            'helper_exit':None,'recheck_result':'unavailable','cause':'unestablished'}
    def created(self,request,row,authority):
        if request is None:return
        if len(self.rows)>=64:self.dropped+=1;return
        value={**request,'ordinal':len(self.rows)+1,'epoch':row['epoch'],'process_ordinal':row['creation_ordinal'],
            'caller_ordinal':row['parent_ordinal'],'authority':authority,'timestamp':row['created_timestamp'],'preceding_close':None}
        if value['operation_class']=='presence_query' and value['predicate_sha256'] and value['caller_ordinal'] is not None:
            prior=next((p for p in reversed(self.rows) if p['epoch']==value['epoch'] and p['caller_ordinal']==value['caller_ordinal'] and p['predicate_sha256']==value['predicate_sha256']),None)
            if prior and prior['operation_class']=='close_request':value['preceding_close']=prior['ordinal']
        self.rows.append(value)
    def exited(self,row):
        for value in self.rows:
            if (value['epoch'],value['process_ordinal'])==(row['epoch'],row['creation_ordinal']):
                value['helper_exit']={k:row['self_exit'][k] for k in ('domain','status')}
    def value(self):return {'schema':1,'observations':self.rows,'dropped_observations':self.dropped,'actual_match_or_close_result':'unavailable_without_exact_object_observation'}


class InstallerWindowsTrace:
    """Observed Wine create/self-exit chain, separate from Linux ownership.

    No PID-number equality joins Windows to Linux. Creation ordinal + launch
    epoch delimit reuse. Paths are launch requests, not claimed mapped identity.
    These are runner-pipe observations, never permission to signal a process.
    """
    HEADER=re.compile(r'^(\d+\.\d+):([0-9a-fA-F]+):([0-9a-fA-F]+):(trace|warn|err|fixme):([a-z0-9_]+):([A-Za-z0-9_]+) (.*)$')
    def __init__(self,artifact,root):
        self.artifact=artifact;self.root=pathlib.Path(root);self.epoch=0;self.pending={};self.rows=[];self.current={};self.seen=set();self.dropped=0;self.first_failure=None;self.cancelled=False;self.image_budget=256*1024*1024;self.service_results=[];self.binding=None;self.presence=InstallerPresence()
    def arm_root(self,operation,token,size):
        if not re.fullmatch('[0-9a-f]{32}',operation) or not re.fullmatch('[0-9a-f]{64}',token):raise ValueError('installer binding identity')
        self.binding={'schema':1,'operation':operation,'epoch':2,'token_sha256':hashlib.sha256(token.encode()).hexdigest(),'artifact_sha256':self.artifact['sha256'],'size':size,'status':'unavailable','reason':'launch_record_not_observed','root_ordinal':None}
        self.binding_token=token;self.binding_frames=0
    def root_frame(self,line):
        if line.startswith(b'IS2_REFUSED_V1 ') and self.binding is not None:
            m=re.fullmatch(rb'IS2_REFUSED_V1 ([0-9a-f]{32}) ([0-9a-f]{64}) 2 ([0-9]{1,10})\r?\n',line)
            if m and m[1].decode()==self.binding['operation'] and m[2].decode()==self.binding_token and int(m[3])<2**32 and self.binding_frames==0 and self.epoch==2:
                self.binding.update(status='unavailable',reason='windows_creation_refused',status_domain='win32_create_process_error',status_code=int(m[3]));self.binding_frames+=1
            return True
        if not line.startswith(b'IS2_ROOT_V1 '):return False
        if self.binding is None:return True
        self.binding_frames+=1
        match=re.fullmatch(rb'IS2_ROOT_V1 ([0-9a-f]{32}) ([0-9a-f]{64}) 2 ([0-9a-f]{64}) ([0-9]{1,10}) ([0-9]{1,10}) ([0-9]{1,20}) ([0-9]{1,10}) ([0-9]{1,20})\r?\n',line)
        b=self.binding
        def refuse(reason):b.update(status='unavailable',reason=reason);return True
        if not match or self.binding_frames!=1 or self.epoch!=2:return refuse('malformed_duplicate_or_wrong_epoch_root')
        op,token,digest,size,pid,created,adapter,adapter_created=[x.decode() for x in match.groups()]
        if (op,token,digest,int(size))!=(b['operation'],self.binding_token,b['artifact_sha256'],b['size']):return refuse('root_binding_mismatch')
        pid,created,adapter,adapter_created=map(int,(pid,created,adapter,adapter_created))
        if not all((pid,created,adapter,adapter_created)) or pid==adapter or max(pid,adapter)>=2**32 or max(created,adapter_created)>=2**64:return refuse('root_generation_invalid')
        row=self.active(pid)
        if pid in self.seen and row is None:return refuse('root_generation_already_retired')
        if row is None:
            if len(self.rows)>=512:return refuse('root_observation_capacity')
            row={'epoch':self.epoch,'creation_ordinal':len(self.rows)+1,'windows_pid':pid,'windows_tid':None,'created_timestamp':None,'parent_ordinal':None,'creator_windows_pid':adapter,'creator_windows_tid':None,'target_tree':False,'target_root':False,'image_request':None,'image_identity':None,'role':'unknown','self_exit':None,'linux_identity':'unavailable_no_cross_id_inference'}
            self.rows.append(row);self.current[pid]=row;self.seen.add(pid)
        elif row['creator_windows_pid']!=adapter or (row['image_identity'] is not None and row['image_identity'].get('sha256')!=b['artifact_sha256']):return refuse('trace_root_creator_or_image_conflict')
        # The adapter proves the mapped image via the retained child handle and
        # file ID. An unavailable/aliased Wine path observation is not stronger
        # than that proof; a positively conflicting identity still refuses.
        row.update(target_root=True,target_tree=True,windows_creation_time=created,root_authority='verified_launch_adapter_exact_handle_image_and_creation_time')
        row['image_identity']={'sha256':digest,'size':int(size),'authority':'launch_adapter_same_open_file_hash_and_child_image_file_id'}
        b.update(status='bound',reason=None,root_ordinal=row['creation_ordinal'],windows_pid=pid,windows_creation_time=created,adapter_windows_pid=adapter,adapter_windows_creation_time=adapter_created)
        return True
    def begin(self):self.epoch+=1;self.pending={};self.current={};self.seen=set()
    def active(self,pid):
        row=self.current.get(pid)
        return row if row and row['epoch']==self.epoch and row['self_exit'] is None else None
    def retire(self,pid):
        self.current.pop(pid,None)
        # Unfinished creates cannot cross this creator generation's retirement.
        for key in [key for key in self.pending if key[0]==pid]:self.pending.pop(key)
    @staticmethod
    def image(body):
        # Decode only the executable prefix. Never retain command arguments/env.
        m=re.match(r'app L"(.{1,2048}?)" cmdline ',body)
        if m:return m[1].replace('\\\\','\\')
        m=re.match(r'app \(null\) cmdline L"\\"(.{1,2048}?)\\"',body)
        if m:return m[1].replace('\\\\','\\')
        return None
    def path(self,windows_path):
        if not windows_path:return None
        if windows_path[:3].lower()=='c:\\':path=self.root/'compatdata/pfx/drive_c'/windows_path[3:].replace('\\','/')
        elif windows_path[:3].lower()=='z:\\':path=pathlib.Path('/'+windows_path[3:].replace('\\','/'))
        else:return None
        if '..' in path.parts:return None
        return path
    def feed(self,line):
        match=self.HEADER.fullmatch(line.decode(errors='replace').rstrip('\r\n'))
        if not match:return False
        ts,pid,tid,level,channel,function,body=match.groups();pid=int(pid,16);tid=int(tid,16);key=(pid,tid)
        if channel=='service' and function=='service_start':
            result=re.fullmatch(r'returning (\d{1,10})',body)
            if result and int(result[1])<2**32:
                if len(self.service_results)<64:self.service_results.append({'epoch':self.epoch,'windows_pid':pid,'windows_tid':tid,'timestamp':ts,'function':function,'status':int(result[1]),'child_association':'unavailable_no_rpc_identity'})
                else:self.dropped+=1
            return True
        if channel=='service' and function in ('CreateServiceW','StartServiceW'):
            row=self.active(pid)
            if row and row['target_tree'] and (self.binding is None or self.binding['status']=='bound'):
                row['role']='service_dependency'
                evidence=row.setdefault('role_evidence',[])
                if len(evidence)<4:evidence.append({'source':'Wine service API observation','function':function,'timestamp':ts})
            return True
        if channel!='process':return False
        if function=='CreateProcessInternalW' and body.startswith('app '):
            if len(self.pending)>=512:self.dropped+=1;return True
            if key in self.pending:self.pending[key]=None # unresolved nesting/refusal cannot authorize pairing
            else:
                parent=self.active(pid)
                self.pending[key]={'timestamp':ts,'image_request':self.image(body),'epoch':self.epoch,
                    'parent_ordinal':parent['creation_ordinal'] if parent else None,'unobserved_creator':pid not in self.seen,'presence':self.presence.request(body)}
            return True
        if function=='NtCreateUserProcess':
            resolved=re.search(r' image L"(.{1,2048}?)" cmdline ',body)
            if resolved and self.pending.get(key) is not None:
                # This is Wine's resolved image request. Do not split an
                # unquoted command line and guess which executable it names.
                self.pending[key]['image_request']=resolved[1].replace('\\\\','\\')
            return True
        created=re.fullmatch(r'started process pid ([0-9a-fA-F]+) tid ([0-9a-fA-F]+)',body) if function=='CreateProcessInternalW' else None
        if created:
            request=self.pending.pop(key,None)
            if len(self.rows)>=512:self.dropped+=1;return True
            child=int(created[1],16);parent=self.active(pid)
            # Creation completion belongs to the same still-active creator that
            # issued the request, not whichever generation now has its PID.
            if not (request and request['epoch']==self.epoch and parent and request['parent_ordinal']==parent['creation_ordinal']):parent=None
            prior=self.active(child)
            if prior:self.dropped+=1;self.retire(child);return True
            path=self.path(request['image_request']) if request else None
            target=bool(self.binding is None and path is not None and path==pathlib.Path(self.artifact['path']) and request['epoch']==self.epoch and request['unobserved_creator'] and pid not in self.seen and parent is None)
            rooted=target or (parent is not None and parent['target_tree'] and (self.binding is None or self.binding['status']=='bound'))
            row={'epoch':self.epoch,'creation_ordinal':len(self.rows)+1,'windows_pid':child,'windows_tid':int(created[2],16),
                 'created_timestamp':ts,'parent_ordinal':parent['creation_ordinal'] if parent else None,
                 'creator_windows_pid':pid,'creator_windows_tid':tid,'target_tree':rooted,'target_root':target,
                 'image_request':request['image_request'] if request else None,'image_identity':None,
                 'role':'unknown','self_exit':None,'linux_identity':'unavailable_no_cross_id_inference'}
            if path is not None:
                try:
                    if path==pathlib.Path(self.artifact['path']):
                        # Import has just been verified by the launch owner. The
                        # launch request names it, but this is not a mapped base.
                        row['image_identity']={'sha256':self.artifact['sha256'],'authority':'verified_import_launch_request'}
                    elif path.is_relative_to(self.root/'compatdata/pfx'):
                        identity=installer_digest(path,min(self.image_budget,64*1024*1024));self.image_budget-=identity['size']
                        row['image_identity']={**identity,'authority':'same_open_file_at_launch_request_not_mapped'}
                except (OSError,ValueError):pass
            self.rows.append(row);self.current[child]=row;self.seen.add(child)
            self.presence.created(request.get('presence') if request else None,row,'bound_installer_descendant' if rooted else 'unbound_runner_trace')
            return True
        exited=re.fullmatch(r'handle (0xffffffffffffffff|0xffffffff), exit_code (-?\d+), process_exiting (1)\.',body) if function=='NtTerminateProcess' else None
        if exited:
            code=int(exited[2]);row=self.active(pid)
            if not row or not -(1<<31)<=code<(1<<32):return True
            value={'domain':'wine_self_exit_observation','status':code&0xffffffff,'timestamp':ts,'source':'Wine NtTerminateProcess self pseudo-handle','process_exiting':int(exited[3])}
            if row['self_exit'] is None:row['self_exit']=value
            elif row['self_exit']['status']!=value['status']:row['exit_conflict']=True;return True
            if value['status'] and row['target_tree'] and (self.binding is None or self.binding['status']=='bound') and not row['target_root'] and self.first_failure is None and not self.cancelled:
                self.first_failure={'role':row['role'],'phase':'target_runner','relationship':'descendant','domain':value['domain'],'status':value['status'],
                    'cause':'unestablished','epoch':self.epoch,'creation_ordinal':row['creation_ordinal'],'timestamp':ts,'windows_pid':pid}
            self.presence.exited(row)
            self.retire(pid)
            return True
        return True # exclude other process lines/arguments from private log projection
    def value(self):return {'schema':1,'processes':self.rows,'dropped_observations':self.dropped,'first_failure':self.first_failure,'service_results':self.service_results,'launch_binding':self.binding,'presence_close':self.presence.value()}

class InstallerTransaction:
    def __init__(self,op,root,report,artifact=None):
        self.windows_trace=InstallerWindowsTrace(artifact or {"path":"/unavailable","sha256":None},root)
        self.op=op;self.root=pathlib.Path(root);self.report=report
        self.path=report.parent/(op+'-transaction-private.json')
        self.witness=InstallerWitnesses(root);self.before=None;self.after=None;self.durable=None
        self.ledger=None;self.private_errors=0;self.accessibility=RecentCapture(32768,64)
        self.ordinary=RecentCapture(262144,256);self.msi=RecentCapture(131072,64);self.line_pending={};self.line_drops=0
        self.image_budget=256*1024*1024;self.image_seen={};self.next_images=0;self.log_tail_bytes=0;self.log_drops=0;self.log_record_drops=0
        self.first_diagnostic=None
    def begin_phase(self):
        self.line_drops+=sum(bool(v[0]) or v[1] for v in self.line_pending.values())
        self.line_pending={};self.windows_trace.begin()
    def persist(self,ledger):
        installer_atomic(self.path,{'schema':1,'operation':self.op,'ledger':ledger,'before':self.before,'after':self.after,
            'durable_installation':self.durable,'diagnostics':self.diagnostics(),'first_diagnostic':self.first_diagnostic,
            'windows_trace':self.windows_trace.value(),
            'stages':[{'process':{'pid':r['pid'],'start_ticks':r['start_ticks']},'classification':r['role'],'evidence':r['role_evidence'],'linux_exit':r['linux_exit'],'windows_exit':r['windows_exit']} for r in ledger['processes']]})
    def diagnostics(self):
        return {'runner':self.ordinary.limits(),'accessibility_observer':self.accessibility.limits(),'msi':self.msi.limits(),
                'line_drops':self.line_drops,'retained_private_log_bytes':self.log_tail_bytes,'dropped_private_log_bytes':self.log_drops,'dropped_private_log_records':self.log_record_drops,
                'unfinished_line_bytes':sum(len(v[0]) for v in self.line_pending.values()),
                'availability':'bounded_runner_pipes','windows_exit_attribution':'bounded_create_self_exit_trace_separate_from_linux_identity'}
    def feed(self,data,stream):
        if stream=='msi':self.msi.write(data);return
        # Stream-local assembly and oversized-line discard. Diagnostic prose can
        # never confer process identity, role or exact exit authority.
        pending,discard=self.line_pending.get(stream,(b'',False));joined=pending+data;pending=b''
        for part in joined.splitlines(keepends=True):
            complete=part.endswith((b'\n',b'\r'))
            if discard:
                if complete:discard=False
                continue
            if len(part)>4096:
                self.line_drops+=1;discard=not complete;continue
            if not complete:pending=part;continue
            pending=b''
            target=self.accessibility if b'Xalia' in part else self.ordinary
            if target is not self.accessibility:
                if stream!='stderr' or not self.windows_trace.root_frame(part):self.windows_trace.feed(part)
            target.write(part)
        self.line_pending[stream]=(b'' if discard else pending,discard)
    def images(self):
        if not self.ledger or time.monotonic_ns()<self.next_images:return
        self.next_images=time.monotonic_ns()+100_000_000
        prefix=self.root/'compatdata/pfx'
        for row in self.ledger.records.values():
            key=(row['pid'],row['start_ticks'])
            if self.image_seen.get(key,0)>=20 or row['state']=='exited':continue
            self.image_seen[key]=self.image_seen.get(key,0)+1
            try:
                proc=pathlib.Path('/proc')/str(key[0])
                if 'linux_executable' not in row:
                    # /proc/PID/exe is a kernel-held image reference, not a
                    # caller pathname. Recheck exact start identity around it.
                    before=self.ledger.checked(key[0])
                    with (proc/'exe').open('rb') as f:
                        md=os.fstat(f.fileno())
                        if stat.S_ISREG(md.st_mode) and md.st_size<=min(self.image_budget,16*1024*1024):
                            digest=hashlib.file_digest(f,'sha256').hexdigest();self.image_budget-=md.st_size
                            after=self.ledger.checked(key[0]);end=os.fstat(f.fileno())
                            stable=(md.st_dev,md.st_ino,md.st_size,md.st_mtime_ns,md.st_ctime_ns)==(end.st_dev,end.st_ino,end.st_size,end.st_mtime_ns,end.st_ctime_ns)
                            if stable and before and after and before['start_ticks']==after['start_ticks']==key[1]:
                                row['linux_executable']={'sha256':digest,'size':md.st_size,'device':md.st_dev,'inode':md.st_ino,'authority':'kernel_exe_reference_exact_start'}
                with (proc/'maps').open('rb') as f:raw=f.read(524289)
                now=self.ledger.checked(key[0])
                if not now or now['start_ticks']!=key[1] or len(raw)>524288:continue
                for line in raw.decode(errors='replace').splitlines():
                    fields=line.split(None,5)
                    if len(fields)!=6 or not fields[5].lower().endswith('.exe'):continue
                    p=pathlib.Path(re.sub(r'\\([0-7]{3})',lambda m:chr(int(m[1],8)),fields[5]))
                    if p==pathlib.Path(self.windows_trace.artifact['path']):relative='imported_installer'
                    else:
                        try:relative=str(p.relative_to(prefix))
                        except ValueError:continue
                    if any(i['location']==relative for i in row['images']):continue
                    if len(row['images'])>=8:break
                    identity=installer_digest(p,min(self.image_budget,64*1024*1024))
                    device=tuple(int(n,16) for n in fields[3].split(':'))
                    if (os.major(identity['device']),os.minor(identity['device']),identity['inode'])!=(*device,int(fields[4])):continue
                    self.image_budget-=identity['size']
                    if any(i['location']==relative and i['sha256']==identity['sha256'] for i in row['images']):continue
                    row['images'].append(dict(identity,location=relative,authority='mapped_device_inode_same_open_digest'))
                    row['image_observation']='observed'
            except (OSError,ValueError):continue
    def finish(self):
        self.after=self.witness.snapshot()
        self.durable=InstallerWitnesses.compare(self.before or {'incomplete':['baseline_unavailable']},self.after)
        # Keep changed log tails private and bounded. Nothing is copied publicly.
        changed_logs=self.durable['delta']['logs']['added']+self.durable['delta']['logs']['changed']
        self.log_record_drops=max(0,len(changed_logs)-32)
        for rel in changed_logs[:32]:
            p=self.witness.prefix/'drive_c'/rel
            try:
                with os.fdopen(installer_open(p),'rb') as f:
                    md=os.fstat(f.fileno())
                    if not stat.S_ISREG(md.st_mode):continue
                    size=min(md.st_size,65536,max(0,1048576-self.log_tail_bytes));f.seek(-size,2)
                    data=f.read(size);self.log_tail_bytes+=len(data);self.log_drops+=md.st_size-len(data)
                name=hashlib.sha256(rel.encode()).hexdigest()
                fd=os.open(self.report.parent/(self.op+'-log-'+name),os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW,0o600)
                with os.fdopen(fd,'wb') as out:out.write(data);out.flush();os.fsync(out.fileno())
            except (OSError,ValueError):self.private_errors+=1
        if self.msi.bytes:
            try:
                with (self.report.parent/(self.op+'-msi-private.log')).open('xb') as f:
                    os.chmod(f.name,0o600)
                    for row in self.msi.rows:f.write(row)
                    f.flush();os.fsync(f.fileno())
            except OSError:self.private_errors+=1
        if self.ledger:self.ledger.commit()
    def summary(self,outer,clean,cancelled,ongoing=False):
        ledger=self.ledger;failure=(self.windows_trace.first_failure if self.windows_trace.binding is None or self.windows_trace.binding['status']=='bound' else None) or (ledger.first_failure if ledger else None)
        durable=self.durable['classification'] if self.durable else 'unavailable'
        if ongoing:outcome='in_progress'
        elif not clean:outcome='cleanup_unconfirmed'
        elif cancelled:outcome='cancelled'
        elif outer not in (None,0) and failure and failure['relationship']=='descendant':outcome='installed_dependency_failed' if durable=='installed' and failure['role']=='service_dependency' else 'child_failed'
        elif outer not in (None,0):outcome='outer_nonzero_stage_unknown'
        elif durable=='installed':outcome='installed'
        elif durable in ('partial_installation','not_installed'):outcome=durable
        else:outcome='completed'
        return {'schema':1,'operation':self.op,'outcome':outcome,'outer_launcher_exit':outer,
            'durable_installation':durable,'installation_completeness':'unproved',
            'first_failure':None if failure is None else {k:failure[k] for k in ('role','phase','relationship','domain','status','cause')},
            'process_count':len(ledger.records) if ledger else 0,
            'windows_self_exit_count':sum(r['self_exit'] is not None for r in self.windows_trace.rows),
            'unavailable_exits':sum(r['linux_exit'] is None for r in ledger.records.values()) if ledger else 0,
            'dropped_process_observations':ledger.dropped if ledger else 0,
            'unattributed_adopted_exits':ledger.unattributed_waits if ledger else 0,
            'private_record_written':self.path.is_file(),'persistence_failures':(ledger.persistence_failures if ledger else 0)+self.private_errors,
            'diagnostics':self.diagnostics(),
            'launch_binding':None if self.windows_trace.binding is None else {k:v for k,v in self.windows_trace.binding.items() if k not in ('windows_pid','windows_creation_time','adapter_windows_pid','adapter_windows_creation_time')},
            'presence_close':{'schema':1,'observation_count':len(self.windows_trace.presence.rows),'dropped_observations':self.windows_trace.presence.dropped,'actual_match_or_close_result':'unavailable_without_exact_object_observation','operation_classes':[v['operation_class'] for v in self.windows_trace.presence.rows]},
            'safe_next_action':'exact_owned_focus_or_stop' if ongoing else 'review_retained_outcome_before_retry' if outcome!='installed' or failure else 'managed_first_launch_required_not_authorized_by_installation'}

class IS3FixtureCapture:
    """Small private oracle sink independent of the rotating diagnostic tail."""
    def __init__(self):self.pending={};self.discard=set();self.rows=[];self.loader=[];self.dropped=0
    def feed(self,data,stream):
        joined=self.pending.pop(stream,b'')+data
        for part in joined.splitlines(keepends=True):
            complete=part.endswith((b'\n',b'\r'))
            if stream in self.discard:
                if complete:self.discard.remove(stream)
                continue
            if len(part)>4096:
                self.dropped+=1
                if not complete:self.discard.add(stream)
                continue
            if not complete:self.pending[stream]=part;continue
            row=part.rstrip(b'\r\n')
            if stream=='stdout' and row.startswith((b'IS3_CAP_V1 ',b'IS3_ENV_V1 ')):
                if len(row)>256 or len(self.rows)>=18:self.dropped+=1
                else:self.rows.append(row.decode('ascii',errors='replace'))
            elif stream=='stderr' and b'powershell.exe' in row.lower() and (b':module:' in row or b':loaddll:' in row):
                if len(self.loader)>=64:self.dropped+=1
                else:self.loader.append(row.decode('utf-8',errors='replace'))
    def value(self):return {'schema':1,'oracle_rows':self.rows,'loader_rows_private':self.loader,'dropped_records':self.dropped,'unfinished_bytes':sum(map(len,self.pending.values()))}


def is3_target_environment(spec, fixture, inherited):
    """Development-only callable seam; no installer spec/CLI/operator authority.

    The source-owned driver supplies an exact operation and independently staged
    fixture artifact. Only one fixed setting, after bootstrap, is permitted.
    """
    if fixture is None:return inherited, None
    if set(fixture)!={'schema','mode','operation','artifact'} or fixture['schema']!=1 or fixture['mode'] not in ('baseline','unix_override','restored'):
        raise RuntimeError('IS3 closed fixture mode')
    if fixture['operation']!=spec['operation'] or fixture['artifact']!=spec['installer'] or spec['format']!='pe_executable':raise RuntimeError('IS3 fixture identity')
    verify(fixture['artifact'])
    changed=dict(inherited)
    if fixture['mode']=='unix_override':changed['WINEDLLOVERRIDES']='powershell.exe='
    changed['WINEDEBUG']+=',trace+loaddll,trace+module'
    value=changed.get('WINEDLLOVERRIDES')
    return changed, {'schema':1,'operation':spec['operation'],'fixture_sha256':fixture['artifact']['sha256'],
        'mode':fixture['mode'],'phase':'target_runner_after_prefix_initialization','monotonic_ns':time.monotonic_ns(),
        'present':value is not None,'value_bytes':len(value.encode()) if value is not None else 0,
        'sha256_utf8':hashlib.sha256((value or '').encode()).hexdigest(),'setting':'powershell.exe=' if fixture['mode']=='unix_override' else 'inherited',
        'authority':'supervisor_Popen_environment','loader_result':'unavailable_without_behavior_or_loader_observation'}


def installer_policy_validate(spec):
    """Closed manager-generated binding; no general environment input capability."""
    policy=spec.get('installer_capability')
    if policy is None:
        if spec.get('schema')==3:raise ValueError('installer_policy_missing')
        return None
    if spec.get('schema')!=3 or not isinstance(policy,dict) or set(policy)!={'schema','operation','environment','installer','software_sha256','software','owners','windows_scripting','format','installer_launch'}:
        raise ValueError('installer_policy_schema')
    if policy['schema']!=1 or policy['operation']!=spec['operation'] or policy['environment']!=spec['environment'] or policy['installer']!=spec['installer']:
        raise ValueError('installer_policy_binding')
    if not re.fullmatch('[a-f0-9]{64}',policy['software_sha256']):raise ValueError('installer_policy_software')
    software=policy['software']
    if hashlib.sha256(json.dumps(software,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()).hexdigest()!=policy['software_sha256']:
        raise ValueError('installer_policy_software_digest')
    if policy['owners']!={k:software[k] for k in ('manager','supervisor','ownership')} or software.get('installer_launch')!=spec.get('installer_launch'):
        raise ValueError('installer_policy_software_owners')
    if policy['format']!='pe_executable' or spec.get('format')!=policy['format']:
        raise ValueError('installer_policy_format')
    adapter=policy['installer_launch']
    if not isinstance(adapter,dict) or set(adapter)!={'path','sha256'} or adapter!=spec.get('installer_launch') or adapter!=software.get('installer_launch'):
        raise ValueError('installer_policy_adapter')
    verify(adapter)
    scripting=policy['windows_scripting']
    if not isinstance(scripting,dict) or set(scripting)!={'powershell'} or scripting['powershell'] not in ('inherited','intentionally_unavailable'):
        raise ValueError('installer_policy_capability')
    owners=policy['owners']
    if not isinstance(owners,dict) or set(owners)!={'manager','supervisor','ownership'}:raise ValueError('installer_policy_owners')
    for artifact in owners.values():
        if not isinstance(artifact,dict) or set(artifact)!={'path','sha256'}:raise ValueError('installer_policy_artifact')
        verify(artifact)
    for name,path in [('supervisor',__file__),('ownership',sys.modules['ownership'].__file__)]:
        actual=pathlib.Path(path)
        if actual.resolve()!=pathlib.Path(owners[name]['path']).resolve():raise ValueError('installer_policy_executing_owner')
        verify({'path':str(actual),'sha256':owners[name]['sha256']})
    return policy

def installer_override_absence(value):
    """Only exact powershell.exe rules are replaced; unrelated segments stay byte exact.

    Multiple exact rules and mixed name lists containing powershell.exe refuse.
    A single case-insensitive rule is replaced in place, or one is appended.
    Never normalize/rewrite unrelated overrides or guess wildcard semantics.
    """
    if value is None:return 'powershell.exe='
    if not isinstance(value,str) or len(value.encode())>16384 or '\0' in value:raise ValueError('installer_override_bound')
    parts=value.split(';');matches=[]
    for i,part in enumerate(parts):
        names,sep,_=part.partition('=')
        keys=[name.strip().casefold() for name in names.split(',')]
        if 'powershell.exe' in keys:
            if not sep or len(keys)!=1:raise ValueError('installer_override_ambiguous_rule')
            matches.append(i)
    if len(matches)>1:raise ValueError('installer_override_duplicate_powershell')
    if matches:parts[matches[0]]='powershell.exe=';return ';'.join(parts)
    return value+('' if not value or value.endswith(';') else ';')+'powershell.exe='

def installer_policy_environment(policy, inherited):
    if policy is None:return inherited,None
    requested=policy['windows_scripting']['powershell']
    changed=dict(inherited)
    if requested=='intentionally_unavailable':
        changed['WINEDLLOVERRIDES']=installer_override_absence(inherited.get('WINEDLLOVERRIDES'))
    def identity(value):
        return {'present':value is not None,'length':len((value or '').encode()),'sha256':hashlib.sha256((value or '').encode()).hexdigest()}
    receipt={'schema':1,'operation':policy['operation'],'environment':policy['environment']['id'],
        'environment_revision':policy['environment']['revision'],'software_sha256':policy['software_sha256'],
        'requested':{'windows_scripting':policy['windows_scripting']},'effective':{'windows_scripting':policy['windows_scripting']},
        'phase':'target_runner_after_prefix_initialization','monotonic_ns':time.monotonic_ns(),
        'before':identity(inherited.get('WINEDLLOVERRIDES')),'after':identity(changed.get('WINEDLLOVERRIDES')),
        'authority':'supervisor_Popen_environment','behavior':'unproved_without_child_observation'}
    return changed,receipt


def managed_install(spec, *, source_owned_is3=None):
    """MF2 initial installer, exact dedicated unit. No product admission authority."""
    op=spec['operation'];env=spec['environment'];root=pathlib.Path(env['root']);report=pathlib.Path(spec['report'])
    if not re.fullmatch('[0-9a-f]{32}',op):raise RuntimeError('installer operation identity')
    lock=(root/'operation.lock').open('a+b');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    scope=None;child=None;stop=False;clean=False;error=None;focus_result=None;diagnostics=RecentCapture(131072,64);private_report_written=False
    sel=selectors.DefaultSelector();start=time.monotonic();startup=InstallerStartup(op,spec['installer'],root)
    transaction=InstallerTransaction(op,root,report,spec['installer']);ledger=None;msi_fd=None;msi_fifo=None;launch_request=None
    policy=None;policy_receipt=None
    is3_capture=IS3FixtureCapture() if source_owned_is3 is not None else None
    def stopping(*_):
        nonlocal stop
        stop=True;startup.cancellation()
        if ledger:ledger.cancelled=True
        transaction.windows_trace.cancelled=True
    signal.signal(signal.SIGTERM,stopping);signal.signal(signal.SIGINT,stopping)
    def reap():
        if ledger:return ledger.harvest()
        return []
    def drain(wait):
        for key,_ in sel.select(wait):
            data=os.read(key.fileobj if isinstance(key.fileobj,int) else key.fileobj.fileno(),65536)
            if data:
                stream='msi' if key.fileobj==msi_fd else 'stdout' if child and key.fileobj is child.stdout else 'stderr'
                if stream!='msi':diagnostics.write(data)
                transaction.feed(data,stream)
                if is3_capture is not None:is3_capture.feed(data,stream)
                if transaction.windows_trace.first_failure and ledger:ledger.commit()
                if stream!='msi':startup.feed(data,stream)
            else:sel.unregister(key.fileobj)
    def outer_exit():
        return child.returncode if child and getattr(child,"installer_phase",None)=="target_runner" else None
    def value(state,live):
        result={'schema':2,'operation':op,'state':state,'raw_exit':outer_exit(),
                'owned_live':live,'cleanup_confirmed':clean,'error':error,'discarded_diagnostic_bytes':diagnostics.dropped_bytes,'retained_diagnostic_bytes':diagnostics.bytes,'private_diagnostics_written':private_report_written,
                'startup':startup.value(),'transaction':transaction.summary(outer_exit(),clean,stop,state in ('starting','running','unknown')),'focus_result':focus_result,'human_action':'installer_ui' if state in ('running','unknown') else None}
        if spec.get('installer_capability') is not None:
            result['installer_capability']={'schema':1,'requested':policy['windows_scripting'] if policy is not None else None,
                'effective':policy_receipt,'operation':op}
        return result
    try:
        policy=installer_policy_validate(spec)
        if policy is not None and source_owned_is3 is not None and source_owned_is3['mode']=='unix_override':
            raise ValueError('installer_policy_cannot_mix_diagnostic_override')
        for a in [spec['installer'],*env['runner']['files']]:verify(a)
        scope=CompanionCgroup(installer_operation=op)
        if scope.members():raise RuntimeError('installer cgroup initially occupied')
        ledger=InstallerLedger(scope,transaction.persist);transaction.ledger=ledger
        if ctypes.CDLL(None,use_errno=True).prctl(36,1,0,0,0)!=0:raise RuntimeError('installer subreaper unavailable')
        runner=env['runner'];base=[runner['entry_point'],'--verb=run','--',runner['proton']]
        argv=base+['runinprefix']
        if spec['format']=='pe_executable':
            adapter=spec.get('installer_launch')
            if adapter is None:argv.append(spec['installer']['path']) # retained schema-2 direct route, no new binding claim
            else:
                verify(adapter);token=os.urandom(32).hex();size=pathlib.Path(spec['installer']['path']).stat().st_size
                transaction.windows_trace.arm_root(op,token,size)
                launch_request=report.parent/(op+'-launch-request.private')
                content='\n'.join(['IS2_LAUNCH_V1',op,token,'2',spec['installer']['sha256'],str(size),windows(spec['installer']['path'],root/'compatdata/pfx'),''])
                with os.fdopen(os.open(launch_request,os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW,0o600),'wb') as f:f.write(content.encode('utf-16le'));f.flush();os.fsync(f.fileno())
                argv.extend([adapter['path'],windows(launch_request,root/'compatdata/pfx')])
        elif spec['format']=='msi_compound':
            msi_fifo=report.parent/(op+'-msi.pipe');os.mkfifo(msi_fifo,0o600)
            msi_fd=os.open(msi_fifo,os.O_RDWR|os.O_NONBLOCK|os.O_NOFOLLOW);sel.register(msi_fd,selectors.EVENT_READ)
            argv+=['msiexec','/i',windows(spec['installer']['path'],root/'compatdata/pfx'),'/L*v',windows(msi_fifo,root/'compatdata/pfx')]
        else:raise RuntimeError('installer format unsupported')
        reg={'environment':env,'compatibility':{'disable_windows_accessibility':False}}
        launch_env=environment(reg);launch_env['HOME']=str(root/'home')
        launch_env['PROTON_LOG']='0'
        launch_env['WINEDEBUG']='-all,+timestamp,+pid,+tid,err+steamclient,err+module,trace+process,trace+service,trace+msi'
        startup.bind_images(runner)
        def launch(args, phase):
            startup.stage(phase+'_launch_requested');transaction.begin_phase()
            startup.line_drops+=sum(bool(v) for v in startup.pending.values())
            startup.pending={};startup.discarding=set()
            process=subprocess.Popen(args,env=launch_env,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True,bufsize=0)
            for pipe in (process.stdout,process.stderr):os.set_blocking(pipe.fileno(),False);sel.register(pipe,selectors.EVENT_READ)
            process.installer_phase=phase;ledger.launcher(process,phase);startup.stage(phase+'_started',pid=process.pid)
            return process
        # This public verb calls init_session(True), including setup_prefix, but
        # never Session.run/steam.exe. runinprefix alone skips setup_prefix.
        child=launch(base+['getcompatpath','/'],'prefix_initialization')
        bootstrap_deadline=time.monotonic()+120
        while child.returncode is None and not stop:
            members=reap();startup.observe(members);transaction.images();drain(.05)
            if time.monotonic()>bootstrap_deadline:
                startup.problem('prefix_initialization_timeout');raise TimeoutError('prefix initialization')
            atomic(report,value('starting',len(scope.members())))
        startup.stage('prefix_initialization_exit',exit=child.returncode)
        if stop:raise InterruptedError('cancelled during prefix initialization')
        if child.returncode!=0:
            startup.problem('prefix_initialization_failed',child.returncode);raise RuntimeError('prefix initialization')
        for _ in range(64):drain(0)
        for pipe in (child.stdout,child.stderr):
            if pipe in sel.get_map():sel.unregister(pipe)
            pipe.close()
        if not (root/'compatdata/pfx/system.reg').is_file():raise RuntimeError('prefix initialization missing')
        # Baseline after runner initialization excludes default prefix construction.
        transaction.before=transaction.witness.snapshot();ledger.commit()
        launch_env,is3_receipt=is3_target_environment(spec,source_owned_is3,launch_env)
        if is3_receipt is not None:atomic(report.parent/(op+'-is3-unix.private.json'),is3_receipt)
        launch_env,prepared_policy=installer_policy_environment(policy,launch_env)
        child=launch(argv,'target_runner')
        # launch() has created and registered the exact owned child. Preparation
        # alone is never public authority that a target launch received policy.
        if prepared_policy is not None:
            prepared_policy['monotonic_ns']=time.monotonic_ns()
            atomic(report.parent/(op+'-installer-policy.private.json'),prepared_policy)
            policy_receipt=prepared_policy
        last=0
        while not stop:
            live=reap();startup.observe(live);transaction.images()
            if dependency_owner is not None and child.returncode is not None:
                # The dependency lives only with this application. Retire the exact
                # cohort after the application adapter exits, never by daemon name.
                clean=ledger.cleanup();live=[] if clean else live
            if dependency_owner and child.returncode is not None:
                clean=ledger.cleanup()
                if not clean:raise ValueError('dependency_retirement_unconfirmed')
                live=ledger.harvest()
            state=vendor_operation_state(child.returncode,len(live))
            if state in ('completed','failed'):
                clean=True
                if state=='failed':error='outer_nonzero_stage_unknown'
                break
            request_path=report.parent/(op+'-focus.json')
            if request_path.exists():
                req=None
                try:
                    with os.fdopen(os.open(request_path,os.O_RDONLY|os.O_NOFOLLOW),'rb') as f:
                        md=os.fstat(f.fileno())
                        if not stat.S_ISREG(md.st_mode) or md.st_uid!=os.getuid() or md.st_size>512:raise RuntimeError('focus request bound')
                        req=json.loads(f.read(513))
                    request_path.unlink()
                    if set(req)!=set(('operation','request')) or req['operation']!=op or not re.fullmatch('[0-9a-f]{32}',req['request']):raise RuntimeError('focus owner changed')
                    result=vendor_focus(scope,None,installer=True)
                except Exception:result='refused_exact_window_unavailable'
                focus_result={'request':req.get('request') if isinstance(req,dict) else None,'result':result}
            if time.monotonic()-start>3600:error='installer_time_bound';break
            if time.monotonic()-last>.5:atomic(report,value(state,len(live)));last=time.monotonic()
            drain(.05)
    except Exception as exc:
        if not stop:
            error='installer_owner_'+type(exc).__name__
            if startup.target is None:startup.problem('supervisor_startup_error')
            else:startup.stage('supervision_error_observed',code=error)
    finally:
        if scope is not None and not clean:
            try:clean=ledger.cleanup() if ledger else not scope.members()
            except Exception:error='installer_cleanup_failed'
        if child:
            for _ in range(64):drain(0)
            child.stdout.close();child.stderr.close()
        if msi_fd is not None:os.close(msi_fd)
        if msi_fifo is not None:
            try:msi_fifo.unlink()
            except FileNotFoundError:pass
        sel.close()
        if launch_request is not None:
            try:launch_request.unlink()
            except FileNotFoundError:pass
        try:transaction.finish()
        except (OSError,ValueError):transaction.private_errors+=1
        state='cleanup_unconfirmed' if not clean else 'cancelled' if stop else 'failed' if error else 'completed'
        try:
            with (report.parent/(op+'-private.log')).open('xb') as private:
                os.chmod(private.name,0o600)
                for data in diagnostics.rows:private.write(data)
                private.flush();os.fsync(private.fileno())
            private_report_written=True
        except OSError:pass # Reporting must not prevent containment or its receipt.
        if is3_capture is not None:atomic(report.parent/(op+'-is3-observation.private.json'),is3_capture.value())
        startup.stage('cohort_retired' if clean else 'cleanup_unconfirmed',outer_exit=outer_exit())
        atomic(report,value(state,0 if clean else None));lock.close()
    return clean and error is None

def vendor_operation_state(launcher_exit, owned_live):
    if launcher_exit is None:return 'running'
    if owned_live:return 'unknown'
    return 'completed' if launcher_exit==0 else 'failed'

class PrivateCapture:
    """Fixed byte/time retention; continue draining after capacity is exhausted."""
    def __init__(self, path, capacity=2*1024*1024, seconds=600):
        self.fd=os.open(path,os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW,0o600)
        self.capacity=capacity;self.deadline=time.monotonic()+seconds
        self.retained=0;self.discarded=0
    def write(self, data):
        count=min(len(data),self.capacity-self.retained) if time.monotonic()<self.deadline else 0
        view=memoryview(data)[:count]
        while view:
            written=os.write(self.fd,view);view=view[written:];self.retained+=written
        self.discarded+=len(data)-count
    def event(self, value):self.write((json.dumps(value,separators=(',',':'))+'\n').encode())
    def close(self):os.close(self.fd)


def vendor_launch(spec):
    app=spec['application'];env=app['environment'];directory=pathlib.Path(env['root']);runner=env['runner']
    mode=spec.get('mode','normal')
    if mode not in ('normal','agent_probe','runinprefix_probe','initialized_probe','accessibility_probe'):raise RuntimeError('vendor launch mode')
    artifact=app['helpers'][0] if mode=='agent_probe' else app['executable']
    executable=pathlib.Path(artifact['path'])
    verb='run' if mode=='initialized_probe' else 'runinprefix'
    argv=[runner['entry_point'],'--verb=run','--',runner['proton'],verb,windows(executable,directory/'compatdata/pfx')]
    return argv,executable.parent


def vendor_diagnostic_environment(env, directory, enabled):
    result=env.copy()
    if enabled:
        # This pinned Proton opens/removes its own unbounded regular log file
        # when PROTON_LOG=1. Keep its redirection off and collect the requested
        # Wine channels through separate, bounded pipes in this private dir.
        result.update(PROTON_LOG='0',PROTON_LOG_DIR=str(directory),
                      WINEDEBUG='-all,+timestamp,+pid,+tid,trace+process,trace+seh,trace+unwind,trace+loaddll,err+module',
                      DXVK_LOG_LEVEL='none',VKD3D_DEBUG='none')
    return result


def vendor_compatibility(mode):
    # Exact registered ASC 2.12 operation tree: the observed UIAutomationCore
    # null-provider crash requires this local selection. Diagnostic comparison
    # modes retain default behavior; no global/prefix registry change.
    if mode not in ('normal','agent_probe','runinprefix_probe','initialized_probe','accessibility_probe'):
        raise RuntimeError('vendor launch mode')
    return {'disable_windows_accessibility':mode in ('normal','accessibility_probe')}


def vendor_process_metadata(scope, record, app):
    root=scope.proc_root/str(record['pid']);roles=[];exe=None
    try:exe=os.readlink(root/'exe')
    except (FileNotFoundError,ProcessLookupError):pass
    candidates=[('main',app['executable']),('agent',app['helpers'][0]),('updater',app['helpers'][1])]
    # Arguments identify an intended launch, not the process running that PE.
    # Match mapped file identity across container path aliases; never read or
    # retain bootstrap arguments that could contain account or URL material.
    try:
        with (root/'maps').open(errors='replace') as f:maps=f.read(262144)
    except (FileNotFoundError,ProcessLookupError):maps=''
    mapped=set()
    for line in maps.splitlines():
        fields=line.split(None,5)
        if len(fields)>=5:
            try:
                major,minor=fields[3].split(':');mapped.add((int(major,16),int(minor,16),int(fields[4])))
            except ValueError:pass
    for role,artifact in candidates:
        path=artifact['path'];m=pathlib.Path(path).stat()
        image_mapped=(os.major(m.st_dev),os.minor(m.st_dev),m.st_ino) in mapped
        if exe==path or image_mapped:
            roles.append({'role':role,'path':path,'sha256':artifact['sha256']})
    if not roles:
        for artifact in app['environment']['runner']['files']:
            if exe==artifact['path']:
                roles.append({'role':'runner_infrastructure','path':artifact['path'],'sha256':artifact['sha256']});break
    parent=scope.identity(record['ppid'])
    return {'executable':exe,'registered_images':roles,'parent_identity':
            {'pid':parent['pid'],'start_ticks':parent['start_ticks']} if parent else None}


def vendor_focus(scope,app,installer=False):
    """Application-origin EWMH focus request, bound to current owned main image.

    No title matching, arbitrary PID, forced input focus or synthetic user input.
    Runs only on the vendor supervisor control path, never the audio path.
    """
    candidates=[]
    for record in scope.members():
        if installer:
            candidates.append(record);continue
        metadata=vendor_process_metadata(scope,record,app)
        if any(v['path']==app['executable']['path'] and v['sha256']==app['executable']['sha256'] for v in metadata['registered_images']):
            candidates.append(record)
    if not candidates or (not installer and len(candidates)!=1):raise RuntimeError('focus main identity ambiguous')
    target=candidates[0]
    def identity():
        now=scope.identity(target['pid'])
        if not now or now['start_ticks']!=target['start_ticks'] or not scope.contains(scope.group_of(target['pid'])):
            raise RuntimeError('focus main identity changed')
    C=ctypes;U=C.c_ulong;I=C.c_int;P=C.c_void_p
    x=C.CDLL('libX11.so.6')
    def bind(name,result,args):f=getattr(x,name);f.restype=result;f.argtypes=args;return f
    bind('XOpenDisplay',P,[C.c_char_p]);bind('XCloseDisplay',I,[P]);bind('XDefaultRootWindow',U,[P]);bind('XInternAtom',U,[P,C.c_char_p,I]);bind('XSync',I,[P,I]);bind('XFree',I,[P])
    bind('XGetWindowProperty',I,[P,U,U,C.c_long,C.c_long,I,U,C.POINTER(U),C.POINTER(I),C.POINTER(U),C.POINTER(U),C.POINTER(P)])
    handler_type=C.CFUNCTYPE(I,P,P);errors=[]
    handler=handler_type(lambda *_:(errors.append(True) if not errors else None) or 0)
    bind('XSetErrorHandler',P,[P]);previous=x.XSetErrorHandler(C.cast(handler,P))
    display=x.XOpenDisplay(None)
    if not display:x.XSetErrorHandler(previous);raise RuntimeError('focus X11 unavailable')
    def prop(window,name):
        kind=U();fmt=I();count=U();after=U();data=P()
        code=x.XGetWindowProperty(display,window,x.XInternAtom(display,name.encode(),0),0,4096,0,0,C.byref(kind),C.byref(fmt),C.byref(count),C.byref(after),C.byref(data))
        try:
            x.XSync(display,0)
            if code or errors or after.value or count.value>4096:raise RuntimeError('focus X11 property unavailable')
            if not kind.value:return []
            if fmt.value!=32:raise RuntimeError('focus property type')
            return list(C.cast(data,C.POINTER(U))[:count.value])
        finally:
            if data:x.XFree(data)
    try:
        root=x.XDefaultRootWindow(display)
        owned={r['pid']:r for r in candidates}
        windows=[(w,prop(w,'_NET_WM_PID')) for w in prop(root,'_NET_CLIENT_LIST') if prop(w,'WM_STATE')[:1]==[1]]
        windows=[(w,ids[0]) for w,ids in windows if len(ids)==1 and ids[0] in owned]
        if len(windows)!=1:raise RuntimeError('focus window absent or ambiguous')
        target=owned[windows[0][1]];windows=[windows[0][0]];identity()
        if len(windows)!=1:raise RuntimeError('focus window absent or ambiguous')
        window=windows[0];identity()
        if prop(window,'_NET_WM_PID')!=[target['pid']]:raise RuntimeError('focus X11 identity changed')
        class Client(C.Structure):
            _fields_=[('type',I),('serial',U),('send_event',I),('display',P),('window',U),('message',U),('format',I),('data',C.c_long*5)]
        class Event(C.Union):_fields_=[('client',Client),('pad',C.c_long*24)]
        event=Event();event.client.type=33;event.client.display=display;event.client.window=window;event.client.message=x.XInternAtom(display,b'_NET_ACTIVE_WINDOW',0);event.client.format=32;event.client.data[0]=1
        bind('XSendEvent',I,[P,U,I,C.c_long,C.POINTER(Event)])
        if not x.XSendEvent(display,root,0,(1<<19)|(1<<20),C.byref(event)):raise RuntimeError('focus request refused')
        deadline=time.monotonic()+.75
        while True:
            identity()
            if prop(root,'_NET_ACTIVE_WINDOW')==[window]:return 'focused'
            if time.monotonic()>=deadline:return 'window_manager_refused'
            time.sleep(.01)
    finally:
        x.XCloseDisplay(display);x.XSetErrorHandler(previous)

# NAUI2 uses the existing Windows trace and Linux ledger as separate authorities.
class RendererImage:
    """One exact large application, one bounded hash, stable open-file cache."""
    def __init__(self,image):
        if set(image)!={'artifact','size'} or type(image['size']) is not int or not 0<image['size']<=256*1024*1024:raise ValueError('renderer_image_bound')
        a=image['artifact'];p=pathlib.Path(a['path'])
        if p.resolve()!=p or set(a)!={'path','sha256'}:raise ValueError('renderer_image_alias')
        self.path=p;self.fd=os.open(p,os.O_RDONLY|os.O_NOFOLLOW);self.closed=False
        try:
            m=os.fstat(self.fd)
            if not stat.S_ISREG(m.st_mode) or m.st_uid!=os.getuid() or m.st_nlink!=1 or m.st_size!=image['size']:raise ValueError('renderer_image_extent')
            self.stamp=self.identity(m);h=hashlib.sha256();left=m.st_size
            while left:
                b=os.read(self.fd,min(left,65536))
                if not b:raise ValueError('renderer_image_short')
                h.update(b);left-=len(b)
            if h.hexdigest()!=a['sha256']:raise ValueError('renderer_image_digest')
            self.sha=a['sha256'];self.size=m.st_size;self.check()
        except BaseException:self.close();raise
    @staticmethod
    def identity(m):return (m.st_dev,m.st_ino,m.st_size,m.st_mtime_ns,m.st_ctime_ns,m.st_nlink)
    def check(self):
        if self.identity(os.fstat(self.fd))!=self.stamp or self.identity(self.path.lstat())!=self.stamp or self.path.resolve()!=self.path:raise ValueError('renderer_image_replaced')
    def close(self):
        if not self.closed:os.close(self.fd);self.closed=True

def renderer_cause(processes,facts,drops,complete,image_sha):
    """Independent closed attribution gate; no inference from incomplete census."""
    if type(drops) is not int or not 0<=drops<=4294967295 or type(complete) is not bool:raise ValueError('renderer_completeness_schema')
    fields={'domain','epoch','ordinal','parent_ordinal','role','role_request_sha256','image_sha256','exit_domain','exit_status'}
    authorities={'chromium_pid_unique_complete_trace':{'gpu','sandbox','renderer','network','graphics_initialization'},
                 'wine_pid_lifetime_complete_trace':{'gdi'},'wine_exact_role_and_self_exit_generation':{'gpu','renderer'}}
    indexed={};last=0
    def sha(v):return isinstance(v,str) and re.fullmatch('[0-9a-f]{64}',v)
    for p in processes:
        if set(p)!=fields or p['domain']!='windows' or p['epoch']!=2 or type(p['ordinal']) is not int or p['ordinal']<=last:raise ValueError('renderer_process_schema')
        last=p['ordinal'];indexed[last]=p
        if p['parent_ordinal'] is not None and (type(p['parent_ordinal']) is not int or not 0<p['parent_ordinal']<last):raise ValueError('renderer_parent_generation')
        if p['role'] not in ('main','gpu','renderer','utility','network','unknown'):raise ValueError('renderer_role')
        if p['role'] not in ('main','unknown') and not sha(p['role_request_sha256']):raise ValueError('renderer_role_request')
        if p['role'] in ('main','unknown') and p['role_request_sha256'] is not None:raise ValueError('renderer_role_request')
        if p['image_sha256'] is not None and not sha(p['image_sha256']):raise ValueError('renderer_process_image')
        if p['exit_status'] is None:
            if p['exit_domain'] is not None:raise ValueError('renderer_exit_domain')
        elif p['exit_domain']!='wine_self_exit_observation' or type(p['exit_status']) is not int or not -2147483648<=p['exit_status']<=4294967295:raise ValueError('renderer_exit_status')
    selected=set()
    for f in facts:
        if set(f)!={'epoch','ordinal','category','authority','record_sha256','image_sha256'} or f['epoch']!=2 or type(f['ordinal']) is not int or not sha(f['record_sha256']) or f['image_sha256']!=image_sha:raise ValueError('renderer_fact_schema')
        p=indexed.get(f['ordinal'])
        if p is None or p['image_sha256']!=image_sha or f['category'] not in authorities.get(f['authority'],set()):raise ValueError('renderer_fact_generation')
        if f['authority']=='wine_exact_role_and_self_exit_generation' and (p['role']!=f['category'] or not p['exit_status']):raise ValueError('renderer_fact_exit')
        if complete and drops==0:selected.add(f['category'])
    selected &= {'gpu','sandbox','renderer','gdi'}
    return next(iter(selected)) if len(selected)==1 else 'unresolved'

class RendererEvidence:
    PATTERNS=(
        ('gpu',r'gpu_process_host\.cc',r'GPU process exited unexpectedly: exit_code=(-?\d{1,10})\.?'),
        ('gpu',r'gpu_data_manager_impl_private\.cc',r"GPU process isn't usable\. Goodbye\."),
        ('sandbox',r'(?:gpu_process_host|child_process_launcher_helper|child_process_launcher_helper_win|sandbox_win)\.cc',r'(?:GPU process launch failed:|Failed to launch child:) error_code=39\.?'),
        ('renderer',r'(?:web_contents|electron_api_web_contents)\.cc',r'render-process-gone: (?:crashed|oom|launch-failed)'),
        ('network',r'network_service_instance_impl\.cc',r'Network service crashed, restarting service\.'),
        ('graphics_initialization',r'(?:gl_surface_egl|gl_display|angle_platform_impl|gpu_init)\.cc',r'(?:ANGLE Display::initialize error [0-9]{1,10}: D3D11 device creation failed|D3D11 device creation failed|OpenGL initialization failed|eglInitialize failed|Failed to initialize GL surface)\.?'),
    )
    CHROMIUM=re.compile(r'^\[([0-9]{1,10}):([0-9]{1,10}):[0-9]{4}/[0-9]{6}\.[0-9]{3,6}:(?:ERROR|FATAL):([a-z_]+\.cc)\([0-9]{1,6}\)\] (.{1,512})$')
    def __init__(self,artifact,root,image):
        self.trace=InstallerWindowsTrace(artifact,root);self.image=image;self.facts=[];self.pending={};self.line_pending={};self.leads=0;self.fact_drops=0;self.line_drops=0
    def begin(self):
        self.trace.begin();self.pending.clear()
        if any(v[0] or v[1] for v in self.line_pending.values()):self.trace.dropped+=1
        self.line_pending.clear()
    @staticmethod
    def role(body):
        m=re.search(r' cmdline L"((?:[^"\\]|\\.)*)", inherit ',body)
        if not m:return 'unknown'
        try:command=json.loads('"'+m[1]+'"')
        except ValueError:return 'unknown'
        values=re.findall(r'(?:^|\s)--type=([a-z-]+)(?=\s|$)',command)
        if len(values)!=1:return 'unknown'
        role={'gpu-process':'gpu','renderer':'renderer','utility':'utility'}.get(values[0],'unknown')
        if role=='utility' and re.findall(r'(?:^|\s)--utility-sub-type=([^\s]+)',command)==['network.mojom.NetworkService']:role='network'
        return role
    def exact(self,row):
        return bool(row and row['epoch']==2 and row['target_tree'] and (row.get('image_identity') or {}).get('sha256')==self.image.sha)
    def add(self,row,category,authority,line):
        if self.trace.cancelled:return
        if not self.exact(row):self.leads+=1;return
        if len(self.facts)>=1024:self.fact_drops+=1;return
        self.facts.append({'epoch':2,'ordinal':row['creation_ordinal'],'category':category,'authority':authority,
            'record_sha256':hashlib.sha256(line).hexdigest(),'image_sha256':self.image.sha})
    def line(self,line,stream):
        t=self.trace
        if line.startswith((b'IS2_ROOT_V1 ',b'IS2_REFUSED_V1 ')):
            if stream!='stderr':t.dropped+=1;return
            t.root_frame(line)
            if t.binding and t.binding['status']=='bound':
                row=t.rows[t.binding['root_ordinal']-1];row['renderer_role']='main'
            return
        m=t.HEADER.fullmatch(line.decode(errors='replace').rstrip('\r\n'))
        before=len(t.rows);request=None;old=None;key=None
        if m:
            ts,pid,tid,level,channel,function,body=m.groups();pid=int(pid,16);key=(pid,int(tid,16))
            old=t.active(pid)
            if channel=='process' and function=='CreateProcessInternalW' and body.startswith('app '):
                # Copy authority from the actual trace pending request below,
                # never associate a later completion with a retired creator.
                role=self.role(body)
                request=(role,hashlib.sha256(line).hexdigest())
            elif channel=='process' and function=='CreateProcessInternalW':request=self.pending.pop(key,None)
        t.feed(line)
        if m and channel=='process' and function=='CreateProcessInternalW' and body.startswith('app '):
            if t.pending.get(key) is not None and request:
                if key in self.pending:self.pending[key]=None
                else:self.pending[key]=request
        if len(t.rows)>before:
            row=t.rows[-1];path=t.path(row.get('image_request'))
            if path==self.image.path:
                self.image.check();row['image_identity']={'sha256':self.image.sha,'size':self.image.size,'authority':'verified_cached_open_file_request'}
            if row['target_tree'] and request and request[0]!='unknown':
                row['renderer_role']=request[0];row['renderer_request_sha256']=request[1]
        if old and old.get('self_exit') is not None:
            for k in [k for k in self.pending if k[0]==old['windows_pid']]:self.pending.pop(k)
            role=old.get('renderer_role')
            if old['self_exit']['status'] and role in ('gpu','renderer'):
                self.add(old,role,'wine_exact_role_and_self_exit_generation',line)
        if m and channel=='gdi' and level=='err' and re.fullmatch(r'out of GDI object handles[.!]?',body,re.I):
            rows=[r for r in t.rows if r['windows_pid']==pid and r['epoch']==2 and r['created_timestamp'] is not None
                and float(r['created_timestamp'])<=float(ts) and (r['self_exit'] is None or float(r['self_exit']['timestamp'])>=float(ts))]
            if len(rows)==1:self.add(rows[0],'gdi','wine_pid_lifetime_complete_trace',line)
            else:self.leads+=1
        c=self.CHROMIUM.fullmatch(line.decode(errors='replace').rstrip('\r\n'))
        if c:
            pid,_,source,body=c.groups()
            for category,filename,pattern in self.PATTERNS:
                if re.fullmatch(filename,source) and re.fullmatch(pattern,body):
                    if 'exit_code=' in body and int(re.search(r'exit_code=(-?\d+)',body)[1])==0:return
                    rows=[r for r in t.rows if r['windows_pid']==int(pid)]
                    if len(rows)==1:self.add(rows[0],category,'chromium_pid_unique_complete_trace',line)
                    else:self.leads+=1
                    break
    def feed(self,data,stream):
        pending,discard=self.line_pending.get(stream,(b'',False))
        for line in (pending+data).splitlines(keepends=True):
            done=line.endswith(b'\n')
            if discard:
                if done:discard=False
                continue
            if len(line)>4096:self.line_drops+=1;self.trace.dropped+=1;discard=not done;continue
            if not done:pending=line;break
            pending=b'';self.line(line,stream)
        self.line_pending[stream]=(b'' if discard else pending,discard)
    def value(self):
        t=self.trace
        incomplete=t.dropped or self.fact_drops or self.line_drops or any(v[0] or v[1] for v in self.line_pending.values())
        complete=not incomplete and t.binding is not None and t.binding['status']=='bound'
        facts=[]
        if complete:
            for f in self.facts:
                row=t.rows[f['ordinal']-1]
                if f['authority']=='chromium_pid_unique_complete_trace' and sum(r['windows_pid']==row['windows_pid'] for r in t.rows)!=1:continue
                facts.append(f)
        result={'schema':1,'windows_dropped_observations':t.dropped,'line_drops':self.line_drops,'fact_drops':self.fact_drops,
            'complete':bool(complete),'unbound_leads':self.leads,'facts':facts,
            'processes':[{'domain':'windows','epoch':r['epoch'],'ordinal':r['creation_ordinal'],
                'parent_ordinal':r['parent_ordinal'],'role':r.get('renderer_role','unknown'),
                'role_request_sha256':r.get('renderer_request_sha256'),
                'image_sha256':(r.get('image_identity') or {}).get('sha256'),
                'exit_domain':r['self_exit']['domain'] if r['self_exit'] else None,
                'exit_status':r['self_exit']['status'] if r['self_exit'] else None} for r in t.rows if r['epoch']==2 and r['target_tree']],
            'launch_binding':None if t.binding is None else {k:t.binding[k] for k in ('schema','operation','epoch','token_sha256','artifact_sha256','size','status','reason','root_ordinal')},'linux_windows_join_performed':False}
        result['cause']=renderer_cause(result['processes'],facts,t.dropped,bool(complete),self.image.sha)
        return result

RENDERER_PRODUCTION = {'environment': '627d2cba97edbecf113c22504eb4c81b', 'installation_operation': 'e4143128adc87de3fdbbdfb44f186ee5', 'observation_sha256': 'de6a9e37b95a6a1917f2749b1a16178c69edcbc6a3a27f639ae0941625395d87', 'source_seal_sha256': '539bcf48c961e18b93ce6bc54c4aed20ab7a82822e7563e34b8025b6836b4be9', 'result_sha256': 'fc9325cd947059e0d453ba69c9a332028197d871e3ef9ab1c94b2fe7d91586a0', 'root_location_sha256': 'f465c845fbb79a281abbcc3fbeae4844b28852fd51624a2a2f2d3383f44a1171', 'files': {'Native Access.exe': {'sha256': '7b2413e90db79538cd1edc7c6169ff384aea181a85635c43a363f4aa18b9e841', 'size': 225757168}, 'chrome_100_percent.pak': {'sha256': 'd76fe8fa3a14cd73e4c858746e2ab2d2fa6f2df38a906fa5814b50a8a895fc96', 'size': 119905}, 'chrome_200_percent.pak': {'sha256': 'b74ad439f3d3762657f3ba533a6af3ac63315c838ed46519891366f43d834a0e', 'size': 197089}, 'icudtl.dat': {'sha256': 'bd8c145abdf3f8383276ce01dfa4ae48709bef9fef1c0711eb7c3fab4f6eb7c2', 'size': 10876560}, 'libEGL.dll': {'sha256': '7d6f4957a6c4d4cad4d717ebebbc9738acf4bfdf2d4ea1c0403d2c9a0661dc5d', 'size': 474624}, 'libGLESv2.dll': {'sha256': '5d6c94812ca1f6dc414d6166da1ed95e4f072a151c7e16952b0fd9a450192d8b', 'size': 8024064}, 'resources.pak': {'sha256': '77f82c006ca57145a385188e67f06c8a1136c0d86de10dc5889f0dbd6117b061', 'size': 7148145}, 'resources/app.asar': {'sha256': '2df87bef2a7c7113b56374d4f5620519d268496bba9414db2e3c101328783ea5', 'size': 76143407}, 'snapshot_blob.bin': {'sha256': '01e68a60b8838be376b202607defadaabdb61b5907dc6caf2cfaf8e915229532', 'size': 365488}, 'v8_context_snapshot.bin': {'sha256': '8b01d2eb0fc5324e0ff3879a30efca0eb7f5dfe35f829721aebc98b0bffbb4c3', 'size': 740048}}}

def renderer_read(path):
    with os.fdopen(os.open(path,os.O_RDONLY|os.O_NOFOLLOW),'rb') as f:
        m=os.fstat(f.fileno())
        if not stat.S_ISREG(m.st_mode) or m.st_uid!=os.getuid() or m.st_nlink!=1 or m.st_size>8*1024*1024:raise ValueError('renderer_record_extent')
        return json.loads(f.read(),object_pairs_hook=renderer_unique)

def renderer_unique(pairs):
    result={}
    for key,value in pairs:
        if key in result:raise ValueError('renderer_duplicate_key')
        result[key]=value
    return result

def renderer_validate(spec, *, dependency=False):
    # Installed entry: no caller-selected home, environment, manifest or fixture.
    if set(spec)!={'schema','kind','application','application_identity','operation','renderer_policy','report','software','software_sha256','installer_launch'} or type(spec['schema']) is not int or spec['schema']!=1 or spec['kind']!=('native_access_dependency' if dependency else 'renderer_application') or spec['renderer_policy'] not in ('inherited','software_rendering'):raise ValueError('renderer_production_schema')
    expected=RENDERER_PRODUCTION
    managed=pathlib.Path.home()/'.local/share/linux-vst-bridge/managed'
    op=spec.get('operation')
    if not isinstance(op,str) or not re.fullmatch('[0-9a-f]{32}',op):raise ValueError('renderer_operation')
    namespace='native-access-dependency' if dependency else 'native-access'
    directory=managed/'vendor-applications'/namespace/'operations'/op
    if spec.get('report')!=str(directory/'result.json') or directory.resolve()!=directory:raise ValueError('renderer_report_location')
    app=spec.get('application',{});env=app.get('environment',{})
    root=managed/'environments'/expected['environment'];drive=root/'compatdata/pfx/drive_c'
    if env.get('id')!=expected['environment'] or env.get('root')!=str(root) or root.resolve()!=root:raise ValueError('renderer_exact_environment')
    if env!=renderer_read(root/'environment.json'):raise ValueError('renderer_environment_record')
    if app.get('id')!='native-access' or app.get('observation_sha256')!=expected['observation_sha256'] or app.get('source_seal_sha256')!=expected['source_seal_sha256']:raise ValueError('renderer_production_provenance')
    files=app.get('files',{})
    if set(files)!=set(expected['files']):raise ValueError('renderer_production_resource_set')
    executable=pathlib.Path(files['Native Access.exe']['artifact']['path']);application=executable.parent
    if not application.is_relative_to(drive) or hashlib.sha256(str(application.relative_to(drive)).encode()).hexdigest()!=expected['root_location_sha256']:raise ValueError('renderer_production_root')
    for name,witness in expected['files'].items():
        if files[name]!={'artifact':{'path':str(application/name),'sha256':witness['sha256']},'size':witness['size']}:raise ValueError('renderer_production_resource')
    installation=managed/'onboarding'/expected['environment']/(expected['installation_operation']+'-result.json')
    if app.get('installation')!={'path':str(installation),'sha256':expected['result_sha256']}:raise ValueError('renderer_production_installation')
    if spec.get('software')!=renderer_read(managed/'software.json'):raise ValueError('renderer_current_software')
    # Persisted request and reservation must name this exact application and operation.
    if renderer_read(directory/'spec.json')!=spec:raise ValueError('renderer_persisted_spec')
    if renderer_read(managed/'vendor-applications'/namespace/'current.json')!={'operation':op,'application':spec['application_identity'],'policy':spec['renderer_policy']}:raise ValueError('renderer_reservation')
    if hashlib.sha256(json.dumps(app,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()).hexdigest()!=spec['application_identity']:raise ValueError('renderer_application_binding')
    renderer_bound_inputs(dict(spec,kind='renderer_application'))
    found=[];pending=[drive];count=0
    while pending:
        for entry in pending.pop().iterdir():
            count+=1
            if count>100000:raise ValueError('renderer_census_bound')
            md=entry.lstat()
            if stat.S_ISDIR(md.st_mode):pending.append(entry)
            if entry.name.lower()=='native access.exe':
                if not stat.S_ISREG(md.st_mode):raise ValueError('renderer_root_alias')
                found.append(entry.parent)
    if found!=[application]:raise ValueError('renderer_unique_root')
    for image in files.values():
        path=pathlib.Path(image['artifact']['path']);before=path.lstat()
        if path.resolve()!=path or not stat.S_ISREG(before.st_mode) or before.st_nlink!=1 or before.st_size!=image['size']:raise ValueError('renderer_resource_bytes')
        verify(image['artifact'])
        if RendererImage.identity(before)!=RendererImage.identity(path.lstat()):raise ValueError('renderer_resource_changed')
    return app

def renderer_bound_inputs(spec):
    if set(spec)!={'schema','kind','application','application_identity','operation','renderer_policy','report','software','software_sha256','installer_launch'} or spec['schema']!=1 or spec['kind']!='renderer_application':raise ValueError('renderer_spec_schema')
    if not re.fullmatch('[0-9a-f]{32}',spec['operation']) or spec['renderer_policy'] not in ('inherited','software_rendering'):raise ValueError('renderer_policy')
    app=spec['application'];sw=spec['software']
    if set(app)!={'schema','id','environment','files','installation','observation_sha256','source_seal_sha256'} or app['schema']!=1:raise ValueError('renderer_application_schema')
    def canonical(v):return json.dumps(v,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()
    if hashlib.sha256(canonical(sw)).hexdigest()!=spec['software_sha256']:raise ValueError('renderer_software_binding')
    if sw.get('installer_launch')!=spec['installer_launch'] or not spec['installer_launch']:raise ValueError('renderer_adapter_binding')
    for name in ('manager','supervisor','ownership','installer_launch'):verify(sw[name])
    for name,path in [('supervisor',__file__),('ownership',sys.modules['ownership'].__file__)]:
        if pathlib.Path(sw[name]['path']).resolve()!=pathlib.Path(path).resolve():raise ValueError('renderer_executing_owner')
    if hashlib.sha256(canonical(app)).hexdigest()!=spec['application_identity']:raise ValueError('renderer_application_binding')
    env=app['environment'];root=pathlib.Path(env['root'])
    if not re.fullmatch('[0-9a-f]{32}',env['id']) or type(env['revision']) is not int or env['revision']<1:raise ValueError('renderer_environment')
    if json.loads((root/'environment.json').read_text())!=env:raise ValueError('renderer_environment_changed')
    total=0
    for name,image in app['files'].items():
        total+=image['size']
        if total>512*1024*1024:raise ValueError('renderer_resource_budget')
        if name!='Native Access.exe':verify(image['artifact'])
    for artifact in env['runner']['files']:verify(artifact)
    verify(app['installation'])
    return app

def renderer_focus(scope,image):
    # Filter Linux window candidates by mapped device/inode and PID/start; never
    # equate a Windows PID with _NET_WM_PID. Existing EWMH owner rechecks custody.
    class ExactImageScope:
        def __getattr__(self,name):return getattr(scope,name)
        def members(self):
            image.check();rows=[]
            dev,ino=image.stamp[:2]
            expected=(os.major(dev),os.minor(dev),ino)
            for r in scope.members():
                try:
                    with (scope.proc_root/str(r['pid'])/'maps').open() as f:lines=f.read(262145)
                    if len(lines)>262144:continue
                    for line in lines.splitlines():
                        fields=line.split(None,5)
                        if len(fields)<5:continue
                        major,minor=fields[3].split(':')
                        if (int(major,16),int(minor,16),int(fields[4]))==expected:
                            now=scope.identity(r['pid'])
                            if now and now['start_ticks']==r['start_ticks']:rows.append(r)
                            break
                except (OSError,ValueError):continue
            return rows
    return vendor_focus(ExactImageScope(),None,installer=True)

def renderer_application(spec):
    # A foreign spec cannot publish even a refusal to its supplied report path.
    renderer_validate(spec)
    dependency=nad1_prepared(spec)
    return renderer_owned(spec,dependency=dependency)

def renderer_owned(spec, preparation=None, dependency=None):
    # Shared mechanism after independent production or sealed-fixture admission.
    # Manager reconciliation takes the same gate before closing an unused launch.
    report=pathlib.Path(spec['report']);op=spec['operation']
    with os.fdopen(os.open(report.parent/'writer.lock',os.O_RDWR|os.O_CREAT|os.O_NOFOLLOW,0o600),'a+b') as gate:
        fcntl.flock(gate,fcntl.LOCK_EX)
        if report.exists():raise ValueError('renderer_operation_already_has_result')
        installer_atomic(report.parent/'writer.json',{'schema':1,'operation':op,'started':True})
        try:
            if preparation is not None:preparation()
            return renderer_run(spec,dependency=dependency)
        except Exception as exc:
            scope=CompanionCgroup(renderer_operation=op);clean=not scope.members()
            atomic(report,{'schema':1,'operation':op,'application_identity':spec['application_identity'],
                'state':'failed' if clean else 'cleanup_unconfirmed','requested':spec['renderer_policy'],
                'effective':None,'outer_exit':None,'cleanup_confirmed':clean,'owned_live':0 if clean else None,
                'cancelled':False,'error':'renderer_admission_'+type(exc).__name__,
                'renderer':{'cause':'unresolved','complete':False}})
            return False


def renderer_run(spec, dependency=None):
    """Exact companion operation; no prefix initialization or installer witnesses."""
    app=spec['application'];op=spec['operation'];env=app['environment'];root=pathlib.Path(env['root']);report=pathlib.Path(spec['report'])
    lock=(root/'operation.lock').open('a+b');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    image=RendererImage(app['files']['Native Access.exe']);evidence=RendererEvidence(app['files']['Native Access.exe']['artifact'],root,image)
    scope=None;ledger=None;child=None;clean=False;stop=False;error=None;effective=None;focus=None;dependency_owner=None
    captures={name:PrivateCapture(report.parent/(op+'-'+name+'.private.log'),16*1024*1024,600) for name in ('stdout','stderr')}
    sel=selectors.DefaultSelector();began=time.monotonic();request_path=report.parent/(op+'-launch.private')
    def persist(value):
        installer_atomic(report.parent/(op+'-ledger.private.json'),{'schema':1,'operation':op,'ledger':value,'windows_trace':evidence.trace.value(),'renderer':evidence.value()})
    def cancel(*_):
        nonlocal stop
        stop=True
        if ledger:ledger.cancelled=True
        evidence.trace.cancelled=True
    def drain(wait):
        for key,_ in sel.select(wait):
            data=os.read(key.fileobj.fileno(),65536)
            if data:captures[key.data].write(data);evidence.feed(data,key.data)
            else:sel.unregister(key.fileobj)
    def confirm_effective():
        nonlocal effective
        binding=evidence.trace.binding
        if binding and binding['status']=='bound' and effective is None:
            effective={'policy':spec['renderer_policy'],'operation':op,'application_identity':spec['application_identity'],
                'software_sha256':spec['software_sha256'],'root_ordinal':binding['root_ordinal'],'monotonic_ns':time.monotonic_ns()}
    def result(state,live):
        return {'schema':1,'operation':op,'application_identity':spec['application_identity'],'state':state,
            'requested':spec['renderer_policy'],'effective':effective,'renderer':evidence.value(),
            'dependency':dependency_owner.value() if dependency_owner else None,
            'outer_exit':child.returncode if child else None,'cleanup_confirmed':clean,'owned_live':live,
            'cancelled':stop,'error':error,'focus_result':focus,
            'application_image_verification':{'bytes':image.size,'hash_passes':1,'cache':'stable_open_file'},
            'diagnostics':{k:{'retained':v.retained,'dropped':v.discarded} for k,v in captures.items()}}
    signal.signal(signal.SIGTERM,cancel);signal.signal(signal.SIGINT,cancel)
    try:
        if not (root/'compatdata/pfx/system.reg').is_file():raise ValueError('renderer_prefix_not_initialized')
        scope=CompanionCgroup(renderer_operation=op)
        if scope.members():raise ValueError('renderer_cgroup_occupied')
        ledger=InstallerLedger(scope,persist)
        if ctypes.CDLL(None,use_errno=True).prctl(36,1,0,0,0)!=0:raise RuntimeError('renderer_subreaper')
        if dependency is not None:
            dependency_owner=Nad1Owner(spec,ledger,scope,lambda:stop)
            dependency_owner.ensure(False,dependency['daemon'])
        token=os.urandom(32).hex();evidence.trace.arm_root(op,token,image.size);evidence.begin();evidence.begin()
        content='\n'.join(['NAUI2_LAUNCH_V1',op,token,'2',image.sha,str(image.size),windows(image.path,root/'compatdata/pfx'),spec['renderer_policy'],''])
        with os.fdopen(os.open(request_path,os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW,0o600),'wb') as f:f.write(content.encode('utf-16le'));f.flush();os.fsync(f.fileno())
        runner=env['runner'];argv=[runner['entry_point'],'--verb=run','--',runner['proton'],'runinprefix',spec['installer_launch']['path'],windows(request_path,root/'compatdata/pfx')]
        launch_env=environment({'environment':env,'compatibility':{'disable_windows_accessibility':False}});launch_env['HOME']=str(root/'home')
        launch_env.update(PROTON_LOG='0',WINEDEBUG='-all,+timestamp,+pid,+tid,trace+process,err+gdi,err+module',DXVK_LOG_LEVEL='none',VKD3D_DEBUG='none')
        image.check()
        child=subprocess.Popen(argv,cwd=image.path.parent,env=launch_env,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True,bufsize=0)
        ledger.launcher(child,'application_runner')
        for name,pipe in [('stdout',child.stdout),('stderr',child.stderr)]:os.set_blocking(pipe.fileno(),False);sel.register(pipe,selectors.EVENT_READ,name)
        last=0
        while not stop:
            live=ledger.harvest();drain(.05);image.check()
            confirm_effective()
            if dependency_owner and child.returncode is not None:
                clean=ledger.cleanup()
                if not clean:raise ValueError('dependency_retirement_unconfirmed')
                live=ledger.harvest()
            state=vendor_operation_state(child.returncode,len(live))
            if state in ('completed','failed'):
                for _ in range(64):drain(0)
                confirm_effective()
                clean=True
                if state=='failed':error='application_outer_nonzero'
                if effective is None:error='application_root_unconfirmed'
                break
            control=report.parent/(op+'-focus.json')
            if control.exists():
                req=None
                try:
                    with os.fdopen(os.open(control,os.O_RDONLY|os.O_NOFOLLOW),'rb') as f:
                        md=os.fstat(f.fileno())
                        if not stat.S_ISREG(md.st_mode) or md.st_uid!=os.getuid() or md.st_size>512:raise ValueError('renderer_focus_bound')
                        req=json.loads(f.read(513))
                    control.unlink()
                    if set(req)!={'operation','request'} or req['operation']!=op or not re.fullmatch('[0-9a-f]{32}',req['request']):raise ValueError('renderer_focus_identity')
                    focus={'request':req['request'],'result':renderer_focus(scope,image)}
                except Exception:focus={'request':req.get('request') if isinstance(req,dict) else None,'result':'refused_exact_window_unavailable'}
            if time.monotonic()-began>600:error='application_time_bound';break
            if time.monotonic()-last>.5:atomic(report,result(state,len(live)));last=time.monotonic()
    except Exception as exc:error='renderer_owner_'+type(exc).__name__
    finally:
        if scope is not None and not clean:
            try:clean=ledger.cleanup() if ledger else not scope.members()
            except Exception:error='renderer_cleanup_failed'
        if child:
            for _ in range(64):drain(0)
            child.stdout.close();child.stderr.close()
        if ledger:ledger.commit()
        sel.close();request_path.unlink(missing_ok=True);image.close()
        for c in captures.values():c.close()
        atomic(report,result('cleanup_unconfirmed' if not clean else 'cancelled' if stop else 'failed' if error else 'completed',0 if clean else None));lock.close()
    return clean and error is None


# NAD1: exact dependency requests use the installed launch adapter's closed SCM
# entry. There is no direct daemon execution and no operator-supplied argument.
NAD1_INSTALLER = 'Program Files/Native Instruments/Native Access/resources/daemon/win/NTKDaemon 1.32.0 Setup PC.exe'
NAD1_INSTALLER_SHA = '5f2199f4e1409d6eea5edaea9c4a8af31e8ee8ac3790851aa44d33e87a46b218'
NAD1_DAEMON = 'Program Files/Common Files/Native Instruments/NTK/NTKDaemon.exe'
NAD1_SERVICE = 'NTKDaemonService'

def nad1_publish(path,value):
    staging=path.with_name(path.name+'.staging-'+os.urandom(8).hex())
    try:
        installer_atomic(staging,value);os.link(staging,path)
        fd=os.open(path.parent,os.O_RDONLY|os.O_DIRECTORY)
        try:os.fsync(fd)
        finally:os.close(fd)
    finally:staging.unlink(missing_ok=True)

def nad1_transition(state):
    table={'absent':'install','unregistered':'install','stopped':'start_service',
           'running_not_ready':'refuse','foreign_conflict':'refuse','ready':'verified_noop','unresolved':'refuse'}
    if state not in table:raise ValueError('dependency_state')
    return table[state]

def nad1_scm_frame(raw,op,token):
    lines=[x.split() for x in raw.decode('ascii',errors='strict').splitlines() if x.startswith('NAD1_SCM_V1 ')]
    if len(lines)!=1 or len(lines[0])!=9:raise ValueError('dependency_scm_frame')
    a=lines[0]
    if a[1:3]!=[op,token] or a[3] not in ('absent','exact') or any(not re.fullmatch('[0-9]{1,20}',v) for v in a[4:8]):raise ValueError('dependency_scm_identity')
    if a[3]=='absent' and a[4:]!=['1060','0','0','0','none']:raise ValueError('dependency_scm_absence')
    if a[3]=='exact' and (int(a[5]) not in range(1,8) or (a[5]=='4' and (not re.fullmatch('[0-9a-f]{64}',a[8]) or int(a[6])==0 or int(a[7])==0))):raise ValueError('dependency_scm_generation')
    return {'registration':a[3],'request_error':int(a[4]),'state':int(a[5]),'windows_pid':int(a[6]),'windows_created':int(a[7]),'image_sha256':a[8]}

def nad1_listener_witness(candidate,scope,proc=pathlib.Path('/proc')):
    # Linux generation/cgroup/descriptor custody only. SCM Windows IDs never enter
    # this function and can never become Linux signal authority.
    import ownership
    pid=candidate['linux_pid'];start=candidate['start_ticks']
    if not any(x['pid']==pid and x['start_ticks']==start for x in scope.members()):raise ValueError('dependency_process_not_owned')
    p=proc/str(pid);before,_=ownership.generation(ownership.bounded(p/'stat','stat'),pid)
    if before!=start:raise ValueError('dependency_process_reused')
    inodes=set();count=0
    with os.scandir(p/'fd') as entries:
        for e in entries:
            count+=1
            if count>8192:raise ValueError('dependency_descriptor_extent')
            try:target=os.readlink(e.path)
            except FileNotFoundError:continue
            m=re.fullmatch(r'socket:\[([0-9]+)\]',target)
            if m:inodes.add(m[1])
    ports=set()
    for name in ('tcp','tcp6'):
        with (p/'net'/name).open('rb') as stream:raw=stream.read(2*1024*1024+1)
        if len(raw)>2*1024*1024 or raw.count(b'\n')>32768:raise ValueError('dependency_listener_extent')
        for line in raw.decode().splitlines()[1:]:
            fields=line.split()
            if len(fields)<10:raise ValueError('dependency_listener_record')
            address,port=fields[1].split(':')
            if fields[3]=='0A' and fields[9] in inodes and address in ('0100007F','00000000000000000000000001000000'):
                ports.add(int(port,16))
    after,_=ownership.generation(ownership.bounded(p/'stat','stat'),pid)
    if after!=start or not any(x['pid']==pid and x['start_ticks']==start for x in scope.members()):raise ValueError('dependency_process_reused')
    return ports.issuperset({5146,5563})

class Nad1Owner:
    def __init__(self,spec,ledger,scope,cancelled,*,fixture=None):
        self.spec=spec;self.ledger=ledger;self.scope=scope;self.cancelled=cancelled
        self.root=pathlib.Path(spec['application']['environment']['root']);self.drive=self.root/'compatdata/pfx/drive_c'
        self.directory=pathlib.Path(spec['report']).parent;self.op=spec['operation'];self.stages=[];self.daemon=None;self.ready=False
        # Only a separately sealed source-owned entry may supply fixture constants.
        self.installer_relative=NAD1_INSTALLER if fixture is None else fixture['installer']
        self.daemon_relative=NAD1_DAEMON if fixture is None else fixture['daemon']
        self.installer_sha=NAD1_INSTALLER_SHA if fixture is None else fixture['installer_sha256']
        self.installer_size=35769456 if fixture is None else fixture['installer_size']
        self.token=os.urandom(32).hex()
    def value(self):
        return {'schema':1,'ready_tested':self.ready,'daemon':self.daemon,
                'stages':self.stages,'readiness_contract':'SCM_exact_image_generation_and_owned_same_prefix_generation_and_both_owned_loopback_listeners_v1',
                'linux_windows_join':False,'lifetime':'owned_operation_only_retired_before_bridge_resume'}
    def command(self,action):
        if action not in ('query','install','start','stop'):raise ValueError('dependency_action')
        index=len(self.stages);request=self.directory/f'{self.op}-dependency-{index}.private'
        content='\n'.join(['NAD1_SERVICE_V1',self.op,self.token,action,self.installer_sha,''])
        with request.open('xb') as f:f.write(content.encode('utf-16le'));f.flush();os.fsync(f.fileno())
        env=self.spec['application']['environment'];runner=env['runner']
        launch_env=environment({'environment':env,'compatibility':{'disable_windows_accessibility':False}})
        launch_env['HOME']=str(self.root/'home');launch_env.update(WINEDEBUG='-all',PROTON_LOG='0')
        argv=[runner['entry_point'],'--verb=run','--',runner['proton'],'runinprefix',self.spec['installer_launch']['path'],windows(request,self.root/'compatdata/pfx')]
        stdout=bytearray();capture=PrivateCapture(self.directory/f'{self.op}-dependency-{index}.log',1024*1024,256);sel=selectors.DefaultSelector()
        stage={'action':action,'launch':'prepared','exit':None,'result':'unavailable'};self.stages.append(stage)
        try:
            child=subprocess.Popen(argv,cwd=self.root/'home',env=launch_env,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True,bufsize=0)
            self.ledger.launcher(child,'dependency_'+action);stage['launch']='owned'
            for name,pipe in [('stdout',child.stdout),('stderr',child.stderr)]:os.set_blocking(pipe.fileno(),False);sel.register(pipe,selectors.EVENT_READ,name)
            deadline=time.monotonic()+(200 if action=='install' else 40)
            while child.returncode is None:
                self.ledger.harvest()
                if self.cancelled():raise ValueError('dependency_cancelled')
                if time.monotonic()>deadline:raise ValueError('dependency_command_timeout')
                for key,_ in sel.select(.05):
                    data=os.read(key.fileobj.fileno(),8192)
                    if not data:sel.unregister(key.fileobj);key.fileobj.close();continue
                    capture.write(data)
                    if key.data=='stdout':
                        if len(stdout)+len(data)>65536:raise ValueError('dependency_helper_output_extent')
                        stdout.extend(data)
            for _ in range(16):
                for key,_ in sel.select(0):
                    data=os.read(key.fileobj.fileno(),8192)
                    if not data:sel.unregister(key.fileobj);key.fileobj.close();continue
                    capture.write(data)
                    if key.data=='stdout':
                        if len(stdout)+len(data)>65536:raise ValueError('dependency_helper_output_extent')
                        stdout.extend(data)
            stage['exit']=child.returncode
            if child.returncode!=0:raise ValueError('dependency_command_nonzero')
            if action=='install':
                expected=f'NAD1_INSTALL_V1 {self.op} {self.token} 0'
                if bytes(stdout).decode().splitlines().count(expected)!=1:raise ValueError('dependency_install_acknowledgment')
                roots=[line.split() for line in bytes(stdout).decode().splitlines() if line.startswith('NAD1_INSTALL_ROOT_V1 ')]
                if len(roots)!=1 or len(roots[0])!=7 or roots[0][1:5]!=[self.op,self.token,self.installer_sha,str(self.installer_size)] or any(not x.isdigit() or int(x)==0 for x in roots[0][5:]):raise ValueError('dependency_installer_root_unbound')
                nad1_publish(self.directory/f'{self.op}-installer-root.private.json',{'frame':roots[0]})
                stage['root_binding']={'operation':self.op,'token_sha256':hashlib.sha256(self.token.encode()).hexdigest(),'sha256':self.installer_sha,'size':self.installer_size,'status':'bound'}
                stage['result']='outer_zero_only';return None
            result=nad1_scm_frame(bytes(stdout),self.op,self.token)
            installer_atomic(self.directory/f'{self.op}-scm-{index}.private.json',result)
            stage['result']=result['registration'];stage['service_state']=result['state'];return result
        finally:
            stage['diagnostic_dropped_bytes']=capture.discarded;capture.close()
            for key in list(sel.get_map().values()):key.fileobj.close()
            sel.close()
            nad1_publish(self.directory/f'{self.op}-dependency-stage-{index}.json',stage)
    def ensure(self,allow_install,admitted=None):
        import ownership
        image=RendererImage({'artifact':{'path':str(self.drive/self.installer_relative),'sha256':self.installer_sha},'size':self.installer_size});image.close()
        path=self.drive/self.daemon_relative
        actual=ownership.image_identity(path) if path.exists() else None
        if admitted is not None and actual!=admitted:raise ValueError('dependency_generation_changed')
        # Presence is not admitted generation identity until this operation installs
        # it or an exact previous preparation receipt supplies its digest.
        if actual is not None and admitted is None:raise ValueError('dependency_existing_unadmitted_generation')
        scan=ownership.census(self.root/'compatdata/pfx',admitted)
        installer_atomic(self.directory/f'{self.op}-dependency-before.private.json',scan)
        if scan['unavailable']:raise ValueError('dependency_identity_unresolved')
        if any(p['prefix_relation'] in ('foreign','deleted') for p in scan['candidates']):raise ValueError('dependency_foreign_conflict')
        service=self.command('query')
        absent=actual is None;unregistered=service['registration']=='absent'
        if absent or unregistered:
            if not allow_install:raise ValueError('dependency_prepare_required')
            self.command('install')
            actual=ownership.image_identity(path)
            service=self.command('query')
            if service['registration']!='exact':raise ValueError('dependency_install_registration_missing')
        self.daemon=actual
        if service['state']==1:service=self.command('start')
        elif service['state'] not in (2,4):raise ValueError('dependency_service_state_unavailable')
        deadline=time.monotonic()+25
        while time.monotonic()<deadline:
            if self.cancelled():raise ValueError('dependency_cancelled')
            self.ledger.harvest();service=self.command('query')
            scan=ownership.census(self.root/'compatdata/pfx',actual)
            installer_atomic(self.directory/f'{self.op}-dependency-current.private.json',scan)
            if scan['unavailable']:raise ValueError('dependency_candidate_ambiguous')
            if any(p['prefix_relation'] in ('foreign','deleted') for p in scan['candidates']):raise ValueError('dependency_foreign_conflict')
            matches=[p for p in scan['private'] if p.get('exact') and p.get('prefix_relation')=='same']
            if service['state']==4 and service['image_sha256']==actual['sha256'] and len(matches)==1 and nad1_listener_witness(matches[0],self.scope):
                self.ready=True;return self.value()
            time.sleep(.1)
        raise ValueError('dependency_running_not_ready')

def nad1_prepared(spec):
    managed=pathlib.Path.home()/'.local/share/linux-vst-bridge/managed';directory=managed/'vendor-applications/native-access-dependency'
    r=renderer_read(directory/'prepared.json')
    if set(r)!={'schema','operation','application','software_sha256','installer_sha256','daemon','result_sha256'} or r['schema']!=1 or r['application']!=spec['application_identity'] or r['software_sha256']!=spec['software_sha256'] or r['installer_sha256']!=NAD1_INSTALLER_SHA or not re.fullmatch('[0-9a-f]{32}',r['operation']):raise ValueError('dependency_prepared_identity')
    receipt=directory/'operations'/r['operation']/'result.json'
    verify({'path':str(receipt),'sha256':r['result_sha256']});v=renderer_read(receipt)
    if v.get('state')!='completed' or not v.get('cleanup_confirmed') or v.get('owned_live')!=0 or v.get('dependency',{}).get('ready_tested') is not True or v['dependency']['daemon']!=r['daemon']:raise ValueError('dependency_prepared_receipt')
    return r

def nad1_owned(spec,*,fixture=None):
    report=pathlib.Path(spec['report']);op=spec['operation'];root=pathlib.Path(spec['application']['environment']['root'])
    with os.fdopen(os.open(report.parent/'writer.lock',os.O_RDWR|os.O_CREAT|os.O_NOFOLLOW,0o600),'a+b') as gate:
        fcntl.flock(gate,fcntl.LOCK_EX)
        if report.exists():raise ValueError('dependency_terminal_exists')
        lock=(root/'operation.lock').open('a+b');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
        scope=CompanionCgroup(dependency_operation=op)
        if scope.members():raise ValueError('dependency_cgroup_occupied')
        ledger=InstallerLedger(scope,lambda v:installer_atomic(report.parent/(op+'-ledger.private.json'),v))
        if ctypes.CDLL(None,use_errno=True).prctl(36,1,0,0,0)!=0:raise RuntimeError('dependency_subreaper')
        stopped=[False]
        def cancel(*_):stopped[0]=True;ledger.cancelled=True
        signal.signal(signal.SIGTERM,cancel);signal.signal(signal.SIGINT,cancel)
        owner=Nad1Owner(spec,ledger,scope,lambda:stopped[0],fixture=fixture);error=None;clean=False
        installer_atomic(report.parent/'writer.json',{'schema':1,'operation':op,'started':True})
        try:
            admitted=fixture.get('admitted') if fixture else None
            if fixture is None:
                try:admitted=nad1_prepared(spec)['daemon']
                except FileNotFoundError:pass
            owner.ensure(True,admitted)
            owner.command('stop')
        except Exception as exc:error=str(exc) if isinstance(exc,ValueError) else 'dependency_owner_'+type(exc).__name__
        finally:
            clean=ledger.cleanup();ledger.commit();lock.close()
            value={'schema':1,'operation':op,'application_identity':spec['application_identity'],'state':'cleanup_unconfirmed' if not clean else 'cancelled' if stopped[0] else 'failed' if error else 'completed','dependency':owner.value(),'cleanup_confirmed':clean,'owned_live':0 if clean else None,'error':error,'cancelled':stopped[0]}
            nad1_publish(report,value)
        return clean and error is None

def nad1_application(spec):
    renderer_validate(spec,dependency=True)
    if spec['renderer_policy']!='software_rendering':raise ValueError('dependency_policy')
    root=pathlib.Path(spec['application']['environment']['root'])
    image=RendererImage({'artifact':{'path':str(root/'compatdata/pfx/drive_c'/NAD1_INSTALLER),'sha256':NAD1_INSTALLER_SHA},'size':35769456});image.close()
    return nad1_owned(spec)


def vendor_application(spec):
    """Own every process in the dedicated unit until observed retirement.

    Linux ancestry is not application-completion authority. Cgroup membership
    survives rapid double-fork, Wine bootstrap and parent replacement. Unknown
    members prevent unit exit just as known main/Agent processes do.
    """
    if spec.get('kind')=='native_access_dependency':return nad1_application(spec)
    if spec.get('kind')=='renderer_application':return renderer_application(spec)
    app=spec['application'];env=app['environment'];directory=pathlib.Path(env['root'])
    report=pathlib.Path(spec['report']);stop=False;child=None;scope=None;clean=False;error=None
    mode=spec.get('mode','normal');diagnostic=mode!='normal';focus_result=None
    lock=(directory/'operation.lock').open('a+b');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    logs=report.parent/('private-diagnostic-'+os.urandom(16).hex());logs.mkdir(mode=0o700)
    captures={name:PrivateCapture(logs/(name+'.log')) for name in ('stdout','stderr','process')}
    journal=captures['process'];sel=selectors.DefaultSelector();observed={};reaped={};last=0
    def event(kind,**fields):journal.event(dict(event=kind,monotonic_ns=time.monotonic_ns(),**fields))
    def cancel(*_):
        nonlocal stop
        stop=True
    signal.signal(signal.SIGTERM,cancel);signal.signal(signal.SIGINT,cancel)
    def reap():
        if child is not None:child.poll()
        for _ in range(128):
            try:info=os.waitid(os.P_ALL,0,os.WEXITED|os.WNOHANG|os.WNOWAIT)
            except ChildProcessError:break
            if info is None:break
            identity=scope.identity(info.si_pid) if scope else None
            pid,status=os.waitpid(info.si_pid,os.WNOHANG)
            if not pid:break
            code=os.waitstatus_to_exitcode(status)
            if len(reaped)<4096:reaped[pid]=code
            event('reaped',pid=pid,start_ticks=identity['start_ticks'] if identity else None,exit_status=code)
    def drain(timeout):
        for key,_ in sel.select(timeout):
            data=os.read(key.fileobj.fileno(),16384)
            if data:captures[key.data].write(data)
            else:sel.unregister(key.fileobj)
    def result(state,live):
        return {'schema':3,'state':state,'operation_id':spec.get('operation_id'),'focus_result':focus_result,'launcher_exit':child.returncode if child else None,
                'owned_live':live,'cleanup_confirmed':clean,'error':error,
                'discarded_diagnostic_bytes':sum(c.discarded for c in captures.values()),
                'retained_diagnostic_bytes':sum(c.retained for c in captures.values()),
                'diagnostic_enabled':diagnostic,'windows_accessibility_disabled':vendor_compatibility(mode)['disable_windows_accessibility'],'account_posture':'unknown'}
    try:
        for artifact in [app['executable'],*app['helpers'],*env['runner']['files']]:verify(artifact)
        scope=CompanionCgroup()
        if scope.members():raise RuntimeError('companion cgroup not initially empty')
        if ctypes.CDLL(None,use_errno=True).prctl(36,1,0,0,0)!=0:raise RuntimeError('companion subreaper unavailable')
        reg={'environment':env,'compatibility':vendor_compatibility(mode)}
        argv,cwd=vendor_launch(spec)
        child=subprocess.Popen(argv,cwd=cwd,env=vendor_diagnostic_environment(environment(reg),logs,diagnostic),
                stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True,bufsize=0)
        event('launcher_started',pid=child.pid,mode=mode,cgroup=scope.group)
        for name,pipe in [('stdout',child.stdout),('stderr',child.stderr)]:
            os.set_blocking(pipe.fileno(),False);sel.register(pipe,selectors.EVENT_READ,name)
        outer_recorded=False
        while not stop:
            members=scope.members();now=time.monotonic_ns()
            for record in members:
                key=(record['pid'],record['start_ticks'])
                if key not in observed:
                    if len(observed)>=4096:raise RuntimeError('companion lifetime process bound')
                    observed[key]={'first_observed':now,'last_observed':now,'metadata':None}
                item=observed[key];item['last_observed']=now
                if item['metadata'] is None or now-item.get('metadata_time',0)>=1_000_000_000:
                    metadata=vendor_process_metadata(scope,record,app);after=scope.identity(record['pid'])
                    if after is None or after['start_ticks']!=record['start_ticks']:continue
                    item['metadata_time']=now
                    if metadata!=item['metadata']:
                        item['metadata']=metadata
                        event('process_observed',**record,**metadata,first_observed=item['first_observed'],last_observed=now)
            present={(v['pid'],v['start_ticks']) for v in members}
            for key,item in observed.items():
                if key not in present and not item.get('retired'):
                    item['retired']=True
                    event('process_disappeared',pid=key[0],start_ticks=key[1],first_observed=item['first_observed'],last_observed=item['last_observed'],exit_status=reaped.get(key[0]))
            reap()
            if child.returncode is not None and not outer_recorded:
                event('launcher_exit',exit_status=child.returncode);outer_recorded=True
            live=[p for p in scope.members() if p['state']!='Z']
            focus_path=report.parent/'focus.json'
            if focus_path.exists():
                request_id=None
                try:
                    with os.fdopen(os.open(focus_path,os.O_RDONLY|os.O_NOFOLLOW),'rb') as f:
                        m=os.fstat(f.fileno())
                        if not stat.S_ISREG(m.st_mode) or m.st_uid!=os.getuid() or m.st_size>1024:raise RuntimeError('focus request bound')
                        request=json.loads(f.read(1025))
                    focus_path.unlink()
                    if not isinstance(request,dict) or set(request)!=set(('request','operation_id')) or not all(isinstance(request[k],str) and re.fullmatch('[0-9a-f]{32}',request[k]) for k in request):
                        raise RuntimeError('focus request schema')
                    request_id=request['request']
                    if request.get('operation_id')!=spec.get('operation_id') or not request.get('operation_id'):
                        raise RuntimeError('focus operation mismatch')
                    focus_result={'request':request['request'],'result':vendor_focus(scope,app)}
                except Exception:
                    focus_result={'request':request_id,'result':'refused_exact_window_unavailable'}
            if dependency_owner and child.returncode is not None:
                clean=ledger.cleanup()
                if not clean:raise ValueError('dependency_retirement_unconfirmed')
                live=ledger.harvest()
            state=vendor_operation_state(child.returncode,len(live))
            if state in ('completed','failed'):
                # No member remains that could create a later handoff. A
                # second ancestry sample or a fixed grace period is not proof.
                for _ in range(64):drain(0)
                confirm_effective()
                clean=True
                if state=='failed':error='vendor_application_launcher_failed_after_cgroup_empty'
                event('cgroup_empty',launcher_exit=child.returncode)
                break
            if time.monotonic()-last>=1:
                atomic(report,result(state,len(live)));last=time.monotonic()
            drain(.1)
    except Exception as exc:
        error='vendor_application_observation_failed'
        event('owner_error',error_type=type(exc).__name__)
    finally:
        if not clean and scope is not None:
            try:clean=scope.cleanup(reap)
            except Exception as exc:event('cleanup_error',error_type=type(exc).__name__)
        if child is not None:
            for _ in range(64):drain(0)
            child.stdout.close();child.stderr.close()
        sel.close()
        state='cleanup_unconfirmed' if not clean else 'failed' if error else 'cancelled' if stop else 'completed'
        event('operation_retired',state=state,cleanup_confirmed=clean)
        atomic(report,result(state,0 if clean else None))
        for capture in captures.values():capture.close()
        lock.close()
    return clean and error is None

if __name__=='__main__':
    os.umask(0o077)
    if sys.argv[1]=='--install':sys.exit(0 if install(json.loads(pathlib.Path(sys.argv[2]).read_text())) else 1)
    if sys.argv[1]=='--vendor-application':sys.exit(0 if vendor_application(json.loads(pathlib.Path(sys.argv[2]).read_text())) else 1)
    spec=json.loads(pathlib.Path(sys.argv[1]).read_text());peer=None if spec['inspect'] or spec.get('vendor_access') else socket.socket(fileno=0)
    operation=(pathlib.Path(spec['registration']['environment']['root'])/'operation.lock').open('a+b')
    # Standalone setup inspection is exclusive: it may start Wine services and
    # must never become their transient owner underneath a live audio instance.
    mode=operation_lock_mode(spec)
    fcntl.flock(operation,mode|fcntl.LOCK_NB)
    try:
        outcome=keep(spec) if spec.get('keeper') else run(spec,peer)
        complete=outcome['cleanup_confirmed'] and (spec.get('keeper') or outcome.get('transport_retired',False))
        # This is an owner result, not the vendor launcher's exit status.
        if complete:print('LVO1 '+spec['session']+' retired',flush=True)
        if outcome.get('reporting_error'):print('Bridge reporting failure: '+outcome['reporting_error'],file=sys.stderr)
        sys.exit(0 if complete else 2)
    finally:
        operation.close()
        if peer is not None:peer.close()

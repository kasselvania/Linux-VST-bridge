#!/usr/bin/env python3
"""One installed instance: existing Windows host, bounded pipes and PID/start cleanup.

No registration writes, checkout imports, scan guesses, global Wine overrides or
callback work. The Rust manager supplies an exact verified registration.
"""
import ctypes,mmap
import fcntl,hashlib,json,os,pathlib,pwd,selectors,shutil,signal,socket,stat,struct,subprocess,sys,time
from ownership import process_identities,descendant_identities,cleanup_process,ProcessTracker,CompanionCgroup

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
    for name in ('ap1.control','ap1.audio','ap11.ui','ap12.status','ap10.delivery','ap18.results'):
        source=directory/name;target=durable/name
        try:m=source.lstat()
        except FileNotFoundError:
            if name in ('ap10.delivery','ap18.results'):continue # retained legacy diagnostic clients
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
        self.result_status=ResultStatus(directory,sid);self.map=None;self.mailbox=None;self.gui=None;self.directory=directory;self.sid=sid;self.last=[None]*3;self.pending=None;self.suspect=None
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
        if self.map is None:return {'available':False,'result_status':self.result_status.snapshot()}
        return {'available':True,'schema':self.version,'sample_monotonic_ns':time.monotonic_ns(),
                'clock_domains':['linux_monotonic_ns','windows_qpc','windows_qpc'],
                **{name:self.lane(i) for i,name in enumerate(('native','delivery','owner'))},
                'editor':self.editor_snapshot(),
                'result_status':self.result_status.snapshot(),
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

def run(spec,peer=None):
    os.umask(0o077);reg=spec['registration'];directory,durable=session_directories(spec);sid=spec['session'];report=pathlib.Path(spec['report'])
    for item in [reg['host'],reg['module'],*reg['environment']['runner']['files']]:verify(item)
    cmd,binding=command(spec);env=environment(reg);transport_environment(spec,env);delivery_trace(spec,env);stop=False
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
            if not line.startswith(b'{"event":'):retain('vendor',vendor,line+b'\n');continue
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
            else:retain('stderr',stderr,data)
    try:
        for stream,label in [(root.stdout,'stdout'),(root.stderr,'stderr')]:os.set_blocking(stream.fileno(),False);sel.register(stream,selectors.EVENT_READ,label)
        tracker=ProcessTracker(root.pid)
        while True:
            owned.update(tracker.update())
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
                failure='Windows SDK host failed: '+str(terminal['exit_code']);break
            if any(r.get('state')=='scanner_completed' for r in records):break
            if root.poll() is not None:
                code=root.returncode
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
                fault={'session':sid,'early_pending':visibility.suspect,'before_containment':visibility.snapshot()}
                atomic(report.with_suffix('.fault.json'),fault)
            except Exception as e:fault_reporting_error=type(e).__name__+': '+str(e)[:256]
            finally:visibility.close()
        try:
            cleanup=cleanup_process(root,sorted(owned));clean=all(cleanup.values())
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
    # Minimal ownership receipt is independent of the rich report, with a
    # bounded stdout result to the live Rust parent even if persistence fails.
    receipt={k:outcome.get(k,False) for k in ('session','cleanup_confirmed','transport_retired')}
    receipt['reporting_error']=outcome.get('reporting_error')
    try:atomic(report.with_suffix('.ownership.json'),receipt)
    except OSError as e:outcome['ownership_reporting_error']=type(e).__name__+': '+str(e)[:256]
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
    env=environment({**reg,'compatibility':{'disable_windows_accessibility':False}});transport_environment(spec,env)
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


def vendor_application(spec):
    """Own every process in the dedicated unit until observed retirement.

    Linux ancestry is not application-completion authority. Cgroup membership
    survives rapid double-fork, Wine bootstrap and parent replacement. Unknown
    members prevent unit exit just as known main/Agent processes do.
    """
    app=spec['application'];env=app['environment'];directory=pathlib.Path(env['root'])
    report=pathlib.Path(spec['report']);stop=False;child=None;scope=None;clean=False;error=None
    mode=spec.get('mode','normal');diagnostic=mode!='normal'
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
        return {'schema':3,'state':state,'launcher_exit':child.returncode if child else None,
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
            state=vendor_operation_state(child.returncode,len(live))
            if state in ('completed','failed'):
                # No member remains that could create a later handoff. A
                # second ancestry sample or a fixed grace period is not proof.
                for _ in range(64):drain(0)
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

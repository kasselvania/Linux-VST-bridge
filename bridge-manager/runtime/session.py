#!/usr/bin/env python3
"""One installed instance: existing Windows host, bounded pipes and PID/start cleanup.

No registration writes, checkout imports, scan guesses, global Wine overrides or
callback work. The Rust manager supplies an exact verified registration.
"""
import ctypes,mmap
import fcntl,hashlib,json,os,pathlib,pwd,selectors,shutil,signal,socket,stat,struct,subprocess,sys,time
from ownership import process_identities,descendant_identities,cleanup_process,ProcessTracker

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
    return env

def command(spec):
    reg=spec['registration'];root=pathlib.Path(reg['environment']['root']);prefix=root/'compatdata/pfx';runner=reg['environment']['runner'];sid=spec['session'];mode='ap12-vendor-access' if spec.get('vendor_access') else 'ap8-module-inspection' if spec['inspect'] else 'ap9-commercial'
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
    """Keep the pinned host's closed C: handshake contract. Only these native-
    created files alias the exact RAM session. Publish before the Windows gate,
    never during processing; owner.json/handshake/receipts remain durable.
    """
    directory,durable=session_directories(spec)
    if directory==durable:return
    sources=[]
    for name in ('ap1.control','ap1.audio','ap11.ui','ap12.status','ap10.delivery'):
        source=directory/name;target=durable/name
        try:m=source.lstat()
        except FileNotFoundError:
            if name=='ap10.delivery':continue # existing explicit socket diagnostic mode
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
        self.map=None;self.mailbox=None;self.gui=None;self.directory=directory;self.sid=sid;self.last=[None]*3;self.pending=None;self.suspect=None
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
        if self.map is None:return {'available':False}
        return {'available':True,'schema':self.version,'sample_monotonic_ns':time.monotonic_ns(),
                'clock_domains':['linux_monotonic_ns','windows_qpc','windows_qpc'],
                **{name:self.lane(i) for i,name in enumerate(('native','delivery','owner'))},
                'editor':self.editor_snapshot(),
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
    visibility=FaultStatus(directory,sid) if not spec['inspect'] and not spec.get('vendor_access') else None
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
            if native_stopped():
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
        outcome={'transport_storage':spec.get('transport'),'fault_status':fault,'fault_reporting_error':fault_reporting_error,'ownership_schema':1,'session':sid,'records':records,'exit_before_cleanup':code,'raw_exit':root.returncode,'error':failure,'cleanup_confirmed':clean,'gated':gated,'discarded_diagnostic_bytes':dropped,'vendor_stdout':vendor.decode(errors='replace'),'stderr':stderr.decode(errors='replace')}
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

def vendor_application(spec):
    """ASC is an exclusive companion operation, never a VST3 instance.

    Launcher exit is not permission to terminate continuing vendor helpers.
    No generic installer deadline or captured vendor log is applied here.
    """
    app=spec['application'];env=app['environment'];directory=pathlib.Path(env['root'])
    lock=(directory/'operation.lock').open('a+b');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    for artifact in [app['executable'],*app['helpers'],*env['runner']['files']]:verify(artifact)
    report=pathlib.Path(spec['report']);stop=False
    def cancel(*_):
        nonlocal stop
        stop=True
    signal.signal(signal.SIGTERM,cancel);signal.signal(signal.SIGINT,cancel)
    runner=env['runner'];reg={'environment':env,'compatibility':{'disable_windows_accessibility':False}}
    argv=[runner['entry_point'],'--verb=run','--',runner['proton'],'runinprefix',windows(app['executable']['path'],directory/'compatdata/pfx')]
    child=subprocess.Popen(argv,env=environment(reg),stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True,bufsize=0)
    tracker=ProcessTracker(child.pid);sel=selectors.DefaultSelector();last=0;discarded=0;error=None;clean=False
    for pipe in (child.stdout,child.stderr):os.set_blocking(pipe.fileno(),False);sel.register(pipe,selectors.EVENT_READ)
    try:
        while not stop:
            owned=tracker.update()
            if len(owned)>4096:raise RuntimeError('vendor application process bound')
            for key,_ in sel.select(.1):
                data=os.read(key.fileobj.fileno(),16384);discarded+=len(data)
                if not data:sel.unregister(key.fileobj)
            if time.monotonic()-last>=1:
                live={(p['pid'],p['start_ticks']) for p in process_identities()} & owned
                state=vendor_operation_state(child.poll(),len(live))
                atomic(report,{'schema':1,'state':state,'launcher_exit':child.returncode,'owned_live':len(live),'discarded_diagnostic_bytes':discarded,'account_posture':'unknown'})
                last=time.monotonic()
                if state in ('completed','failed'):
                    clean=True
                    if state=='failed':error='vendor application exited unsuccessfully'
                    break
    except Exception as e:error=type(e).__name__+': '+str(e)
    finally:
        if stop or error:
            try:clean=all(cleanup_process(child,sorted(tracker.owned)).values())
            except Exception as e:error=type(e).__name__+': '+str(e)
        sel.close();child.stdout.close();child.stderr.close()
        atomic(report,{'schema':1,'state':'cleanup_unconfirmed' if not clean else 'failed' if error else 'cancelled' if stop else 'completed','cleanup_confirmed':clean,'error':error,'launcher_exit':child.returncode,'discarded_diagnostic_bytes':discarded,'account_posture':'unknown'})
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

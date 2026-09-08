#!/usr/bin/env python3
"""One installed instance: existing Windows host, bounded pipes and PID/start cleanup.

No registration writes, checkout imports, scan guesses, global Wine overrides or
callback work. The Rust manager supplies an exact verified registration.
"""
import fcntl,hashlib,json,os,pathlib,pwd,selectors,shutil,signal,socket,stat,struct,subprocess,sys,time
from ownership import process_identities,descendant_identities,cleanup_process

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
    pairs=[('session',sid),('scanner-sha256',reg['host']['sha256']),('implementation-source-manifest-sha256',reg['host_source_sha256']),
           ('module',windows(reg['module']['path'],prefix)),('module-sha256',reg['module']['sha256']),('bundle-manifest-sha256',reg['module']['sha256']),
           ('ready',f'C:\\bridge\\sessions\\{sid}\\{sid}.ready'),('gate',f'C:\\bridge\\sessions\\{sid}\\{sid}.gate'),
           ('max-classes','256'),('stdout-cap','1048576'),('mode',mode),('component-case',case)]
    cmd=[runner['entry_point'],'--verb=run','--',runner['proton'],'runinprefix',windows(reg['host']['path'],prefix)]
    for k,v in pairs:cmd+=['--'+k,v]
    binding=('schema=linux-vst-bridge-wf0-handshake/v1\n'+''.join(k.replace('-','_')+'='+v+'\n' for k,v in [pairs[0],pairs[1],pairs[4],pairs[5],pairs[2],pairs[10],pairs[11]])+'run_ordinal=1\n').encode()
    return cmd,binding

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

def run(spec,peer=None):
    os.umask(0o077);reg=spec['registration'];directory=pathlib.Path(spec['directory']);sid=spec['session'];report=pathlib.Path(spec['report']);expected_dir=pathlib.Path(reg['environment']['root'])/'compatdata/pfx/drive_c/bridge/sessions'/sid
    if directory!=expected_dir or len(sid)!=32 or any(c not in '0123456789abcdef' for c in sid):raise RuntimeError('session binding differs')
    if directory.is_symlink() or not directory.is_dir() or directory.stat().st_mode&0o077:raise RuntimeError('session directory is not private')
    for item in [reg['host'],reg['module'],*reg['environment']['runner']['files']]:verify(item)
    cmd,binding=command(spec);env=environment(reg);delivery_trace(spec,env);stop=False
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
        while True:
            census=process_identities()
            for r in census:
                if r['pid']==root.pid:owned.add((r['pid'],r['start_ticks']))
            owned.update((r['pid'],r['start_ticks']) for r in descendant_identities(root.pid,census))
            pump(.05)
            if native_stopped():
                if spec.get('vendor_access'):
                    (directory/'vendor.stop').write_text(sid+'\n')
                    end=time.monotonic()+10
                    while time.monotonic()<end and not any(r.get('state')=='scanner_completed' for r in records):pump(.05)
                break
            if not gated and any(r.get('state')=='readiness_announced' for r in records):
                ready=directory/(sid+'.ready');m=ready.lstat()
                if not stat.S_ISREG(m.st_mode) or m.st_size>1024 or ready.read_bytes()!=binding:raise RuntimeError('Windows readiness binding differs')
                for item in [reg['host'],reg['module']]:verify(item)
                gate=directory/(sid+'.gate');temp=directory/(sid+'.gate.tmp')
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
        try:
            cleanup=cleanup_process(root,sorted(owned));clean=all(cleanup.values())
        except Exception as e:
            failure=(failure+'; ' if failure else '')+'cleanup: '+str(e)
        sel.close()
        for stream in (root.stdout,root.stderr):stream.close()
        outcome={'ownership_schema':1,'session':sid,'records':records,'exit_before_cleanup':code,'raw_exit':root.returncode,'error':failure,'cleanup_confirmed':clean,'gated':gated,'discarded_diagnostic_bytes':dropped,'vendor_stdout':vendor.decode(errors='replace'),'stderr':stderr.decode(errors='replace')}
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
            shutil.rmtree(directory)
            if peer is not None:
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
    root=subprocess.Popen(cmd,env=environment({**reg,'compatibility':{'disable_windows_accessibility':False}}),stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True,bufsize=0)
    sel=selectors.DefaultSelector();owned=set();text=bytearray();ready=False;started=time.monotonic();error=None;clean=False
    for pipe in (root.stdout,root.stderr):os.set_blocking(pipe.fileno(),False);sel.register(pipe,selectors.EVENT_READ)
    try:
        while not stop:
            census=process_identities()
            owned.update((r['pid'],r['start_ticks']) for r in census if r['pid']==root.pid)
            owned.update((r['pid'],r['start_ticks']) for r in descendant_identities(root.pid,census))
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

if __name__=='__main__':
    os.umask(0o077)
    if sys.argv[1]=='--install':sys.exit(0 if install(json.loads(pathlib.Path(sys.argv[2]).read_text())) else 1)
    spec=json.loads(pathlib.Path(sys.argv[1]).read_text());peer=None if spec['inspect'] or spec.get('vendor_access') else socket.socket(fileno=0)
    operation=(pathlib.Path(spec['registration']['environment']['root'])/'operation.lock').open('a+b')
    # Standalone setup inspection is exclusive: it may start Wine services and
    # must never become their transient owner underneath a live audio instance.
    mode=fcntl.LOCK_EX if spec['inspect'] and not spec.get('keeper') else fcntl.LOCK_SH
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

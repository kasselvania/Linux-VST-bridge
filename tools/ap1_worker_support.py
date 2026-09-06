"""AP1 companion process uses the accepted supervisor and owned cleanup primitive."""
import json,pathlib,os,secrets,subprocess,time,hashlib,re,copy
import supervise as inherited
import pc0_diagnostic_primitives as diagnostic
import ap0_worker_support as ap0
from ap1_contract import MODE,normalize,validate_summary
from ap1_runtime import verify_runtime
from ap1_client_artifact import verify_client
from ap0_artifacts import verify_host_store
from pc0_diagnostic_runtime import exception_detail,sanitized_supervision_error

_client=None

def client_root(binding):
    from common import real_home
    return real_home()/'.local/share/linux-vst-bridge/client-artifacts/by-manifest'/binding['manifest_sha256']

def bind_client(binding):
    global _client
    root=client_root(binding);verify_client(root,binding)
    if not os.access(root/'ap1-native-client',os.X_OK):raise RuntimeError('AP1 caller is not executable')
    for path in pathlib.Path('/proc').glob('[0-9]*/cmdline'):
        try:raw=path.read_bytes()[:16384]
        except (FileNotFoundError,PermissionError,ProcessLookupError):continue
        if b'ap1-native-client' in raw and b'--session-dir' in raw:raise RuntimeError('prior native AP1 caller is still present')
    _client=root/'ap1-native-client'

def verify_host(root,identity):
    return verify_host_store(root,identity,expected_branch='codex/ap1-linux-windows-audio-roundtrip',expected_input_count=22)

def command_vector(environment,session,component_case,mode):
    if mode!=MODE:raise RuntimeError('AP1 mode differs')
    v=inherited.command_vector(environment,session,component_case,inherited.PC0_MODE)
    v[v.index('--mode')+1]=MODE;return v

StreamState=ap0.StreamState

def retained_stream(path):
    """Hash the stream without retaining it; keep only a bounded error excerpt."""
    if path.is_symlink() or not path.is_file():raise RuntimeError('unsafe caller diagnostic stream')
    digest=hashlib.sha256();tail=b''
    with path.open('rb') as stream:
        # Snapshot the current extent. A still-live child cannot extend this read.
        remaining=os.fstat(stream.fileno()).st_size
        if remaining>8*1024*1024:raise RuntimeError('caller diagnostic stream exceeds read bound')
        while remaining:
            block=stream.read(min(65536,remaining))
            if not block:raise RuntimeError('caller diagnostic stream truncated during read')
            remaining-=len(block);digest.update(block);tail=(tail+block)[-8192:]
    lines=[line for line in tail.decode('utf-8','replace').splitlines()
           if re.search(r'(?i)\b(error|exception|fatal|assertion|aborted|crash|terminate|what)\b',line)]
    detail=sanitized_supervision_error(RuntimeError(' | '.join(lines[-4:]))) if lines else None
    return digest.hexdigest(),detail

def supervise(environment,*,mode,checkpoint,profile,caller_command=None,caller_env=None,windows_run=None,accepted_events=None,session=None,ready_seconds=5,exit_seconds=2,caller_report=None,track_descendants=False,post_gate_seconds=120,caller_checkpoint_report=None):
    if _client is None and caller_command is None:raise RuntimeError('native caller was not admitted')
    session=session or secrets.token_hex(16)
    out_path=environment.session/'ap1-client.jsonl';err_path=environment.session/'ap1-client.stderr'
    client=None;identity=None;observed=None;primary=None;windows_started=False;caller={'records':[],'raw_exit':None,'cleanup':{'owned_descendants_zero':False,'process_group_empty':False}}
    windows_cleanup={'owned_descendants_zero':False,'process_group_empty':False}
    captured={}
    seen_caller=set()
    def failed(field,error):
        nonlocal primary
        caller.setdefault(field,exception_detail(error))
        primary=primary or error
    def snapshot():
        available=observed or captured or {'records':[],'classification':'native_setup_failed','raw_exit':None}
        return {**available,'caller':copy.deepcopy(caller),'cleanup':{
            k:windows_cleanup.get(k) is True and caller['cleanup'][k] is True for k in windows_cleanup}}
    def read_report(reader):
        records=[]
        try:
            if reader is None:
                with out_path.open('rb') as stream:raw=stream.read(192*1024+1)
            else:raw=reader()
            if len(raw)>192*1024:raise RuntimeError('native report exceeds bound')
            for line in raw.splitlines():
                value=json.loads(line)
                if not isinstance(value,dict) or value.get('event') not in (accepted_events or {'ap1_client_ready','ap1_client_block','ap1_client_closed','ap1_client_error'}):raise RuntimeError('native report event differs')
                records.append(value)
            if raw and not raw.endswith(b'\n'):raise RuntimeError('native report was truncated')
        except Exception as error:
            # Retain a valid prefix, or an earlier complete checkpoint if the
            # later GUI-note/report path fails. This is never product admission.
            if len(records)>len(caller['records']):caller['records']=records
            failed('reporting_error',error)
        else:caller['records']=records
    def retain_caller():
        # Capture native facts before cleanup. GUI-note waiting stays AFTER
        # owned cleanup, using the GUI helper's cached native bytes.
        read_report(caller_checkpoint_report or caller_report)
        for name,path in [('stderr',err_path),*([('stdout',out_path)] if caller_report is not None else [])]:
            try:
                digest,detail=retained_stream(path);caller[name+'_sha256']=digest
                if caller_command is not None and detail is not None:caller[name+'_detail']=detail
            except Exception as error:failed('retention_error',error)
        if caller_report is not None:
            # Bitwig may leave this fixed-name text report in its disposable cwd.
            # No directory scan, core dump, private environment or raw log export.
            crash=environment.session/'BITWIG_ENGINE_CRASH.txt'
            if crash.exists() or crash.is_symlink():
                try:
                    digest,detail=retained_stream(crash);caller['crash_sha256']=digest
                    if detail is not None:caller['crash_detail']=detail
                except Exception as error:failed('retention_error',error)
        if primary is not None:caller['error']=exception_detail(primary)
        checkpoint('native_caller_before_cleanup',snapshot(),primary)
    def observe_caller():
        if track_descendants and client is not None:
            for process in inherited.descendants(client.pid):
                seen_caller.add((process['pid'],process['start_ticks']))
    def capture(stage,available=None,error=None):
        nonlocal windows_cleanup,captured
        if available is not None:
            captured={**captured,**available}
            if 'cleanup' in available:windows_cleanup=available['cleanup']
        # The Windows group alone cannot authorize retirement while Linux lives.
        projected={**captured,'cleanup':{'owned_descendants_zero':False,'process_group_empty':False}} if available else None
        checkpoint(stage,projected,error)
    with out_path.open('xb') as out,err_path.open('xb') as err:
        try:
            client=subprocess.Popen(caller_command or [str(_client),'--session-dir','.','--session',session],cwd=environment.session,
                stdin=subprocess.DEVNULL,stdout=out,stderr=err,start_new_session=True,
                env=caller_env or inherited.controlled_environment(environment))
            identity=inherited.process_identity(client.pid)
            seen_caller.add((identity['pid'],identity['start_ticks']))
            deadline=time.monotonic()+ready_seconds
            while not (environment.session/'ap1.control').exists():
                observe_caller()
                if client.poll() is not None:raise RuntimeError('native caller exited during setup')
                if time.monotonic()>deadline:raise RuntimeError('native caller setup timeout')
                time.sleep(.01)
            windows_started=True
            observed=(windows_run(capture) if windows_run else diagnostic.supervise(environment,mode=mode,checkpoint=capture,profile=profile,session_override=session,observe_companion=observe_caller,post_gate_seconds=post_gate_seconds))
            windows_cleanup=observed['cleanup']
        except Exception as error:
            primary=error
        finally:
            if not windows_started:windows_cleanup={'owned_descendants_zero':True,'process_group_empty':True}
            if client is not None:
                try:
                    end=time.monotonic()+exit_seconds
                    while client.poll() is None and time.monotonic()<end:
                        observe_caller();time.sleep(.05)
                except Exception as error:failed('cleanup_error',error)
                caller['raw_exit']=client.poll()
                if caller['raw_exit'] is None:primary=primary or RuntimeError('native caller exit timeout after Windows completion')
            # Retention/checkpoint errors must not prevent owned-process cleanup.
            try:
                out.flush();err.flush();retain_caller()
            except Exception as error:failed('retention_error',error)
            if client is not None:
                # Same PID/start-time and process-group cleanup used for Windows.
                try:
                    caller['cleanup']=inherited.cleanup_process(client,sorted(seen_caller))
                except Exception as error:
                    failed('cleanup_error',error)
            else:
                caller['cleanup']={'owned_descendants_zero':True,'process_group_empty':True}
                windows_cleanup=caller['cleanup']
    if primary is not None:caller['error']=exception_detail(primary)
    if caller_checkpoint_report is not None:
        read_report(caller_report)
        if primary is not None:caller['error']=exception_detail(primary)
    available=snapshot()
    checkpoint('native_caller_retained',available,primary)
    if primary is not None:raise primary
    return available

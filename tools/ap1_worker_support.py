"""AP1 companion process uses the accepted supervisor and owned cleanup primitive."""
import json,pathlib,os,secrets,subprocess,time
import supervise as inherited
import pc0_diagnostic_primitives as diagnostic
import ap0_worker_support as ap0
from ap1_contract import MODE,normalize,validate_summary
from ap1_client_artifact import verify_client
from ap0_artifacts import verify_host_store
from pc0_diagnostic_runtime import exception_detail

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

def supervise(environment,*,mode,checkpoint,profile):
    if _client is None:raise RuntimeError('native caller was not admitted')
    session=secrets.token_hex(16)
    out_path=environment.session/'ap1-client.jsonl';err_path=environment.session/'ap1-client.stderr'
    client=None;identity=None;observed=None;primary=None;windows_started=False;caller={'records':[],'raw_exit':None,'cleanup':{'owned_descendants_zero':False,'process_group_empty':False}}
    windows_cleanup={'owned_descendants_zero':False,'process_group_empty':False}
    captured={}
    def capture(stage,available=None,error=None):
        nonlocal windows_cleanup,captured
        if available is not None:
            captured={**captured,**available}
            if 'cleanup' in available:windows_cleanup=available['cleanup']
        # The Windows group alone cannot authorize retirement while Linux lives.
        projected={**available,'cleanup':{'owned_descendants_zero':False,'process_group_empty':False}} if available else None
        checkpoint(stage,projected,error)
    with out_path.open('xb') as out,err_path.open('xb') as err:
        try:
            client=subprocess.Popen([str(_client),'--session-dir','.','--session',session],cwd=environment.session,
                stdin=subprocess.DEVNULL,stdout=out,stderr=err,start_new_session=True,
                env=inherited.controlled_environment(environment))
            identity=inherited.process_identity(client.pid)
            deadline=time.monotonic()+5
            while not (environment.session/'ap1.control').exists():
                if client.poll() is not None:raise RuntimeError('native caller exited during setup')
                if time.monotonic()>deadline:raise RuntimeError('native caller setup timeout')
                time.sleep(.01)
            windows_started=True
            observed=diagnostic.supervise(environment,mode=mode,checkpoint=capture,profile=profile,session_override=session)
            windows_cleanup=observed['cleanup']
        except Exception as error:
            primary=error
        finally:
            if client is not None:
                try:client.wait(timeout=2)
                except subprocess.TimeoutExpired:pass
                caller['raw_exit']=client.poll()
                # Same PID/start-time and process-group cleanup used for Windows.
                try:
                    caller['cleanup']=inherited.cleanup_process(client,[] if identity is None else [(identity['pid'],identity['start_ticks'])])
                except Exception as error:
                    primary=primary or error
            else:
                caller['cleanup']={'owned_descendants_zero':True,'process_group_empty':True}
                windows_cleanup=caller['cleanup']
    if not windows_started:windows_cleanup={'owned_descendants_zero':True,'process_group_empty':True}
    import hashlib
    caller['stderr_sha256']=hashlib.sha256(err_path.read_bytes()).hexdigest()
    try:
        raw=out_path.read_bytes()
        if len(raw)>192*1024:raise RuntimeError('native report exceeds bound')
        lines=raw.splitlines()
        for line in lines:
            value=json.loads(line)
            if not isinstance(value,dict) or value.get('event') not in {'ap1_client_ready','ap1_client_block','ap1_client_closed','ap1_client_error'}:raise RuntimeError('native report event differs')
            caller['records'].append(value)
        if raw and not raw.endswith(b'\n'):raise RuntimeError('native report was truncated')
    except Exception as error:primary=primary or error
    if primary is not None:caller['error']=exception_detail(primary)
    available=observed or captured or {'records':[],'classification':'native_setup_failed','raw_exit':None}
    available={**available,'caller':caller,'cleanup':{k:windows_cleanup.get(k) is True and caller['cleanup'][k] is True for k in windows_cleanup}}
    checkpoint('native_caller_retained',available,primary)
    if primary is not None:raise primary
    return available

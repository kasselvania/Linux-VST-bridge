"""AP2 SDK host is a companion of the existing external Windows supervisor."""
import pathlib,os,secrets,time,json
import supervise as inherited
import pc0_diagnostic_primitives as diagnostic
import ap1_worker_support as companion
from ap0_worker_support import StreamState
from ap1_runtime import verify_runtime
from ap0_artifacts import verify_host_store
from ap2_native_artifact import verify_native
from ap2_contract import MODE,normalize,validate_summary
_native=None

def verify_host(root,identity):
 return verify_host_store(root,identity,expected_branch='codex/ap2-native-vst3-offline-bridge',expected_input_count=22)
def bind_client(binding):
 global _native
 from common import real_home
 root=real_home()/'.local/share/linux-vst-bridge/native-artifacts/by-manifest'/binding['manifest_sha256']
 verify_native(root,binding)
 if not os.access(root/'ap2-offline-host',os.X_OK):raise RuntimeError('AP2 SDK host is not executable')
 for p in pathlib.Path('/proc').glob('[0-9]*/cmdline'):
  try:raw=p.read_bytes()[:16384]
  except (FileNotFoundError,PermissionError,ProcessLookupError):continue
  if b'ap2-offline-host' in raw:raise RuntimeError('prior AP2 host is present')
 _native=root

def command_vector(environment,session,component_case,mode):
 if mode!=MODE:raise RuntimeError('AP2 mode differs')
 v=inherited.command_vector(environment,session,component_case,inherited.PC0_MODE)
 v[v.index('--mode')+1]=MODE;return v

def supervise(environment,*,mode,checkpoint,profile):
 if _native is None:raise RuntimeError('native AP2 artifact not admitted')
 sessions=[secrets.token_hex(16),secrets.token_hex(16)]
 env={**inherited.controlled_environment(environment),'LVB_AP2_SESSION_DIR':str(environment.session),
      'LVB_AP2_SESSION':sessions[0],'LVB_AP2_REOPEN_SESSION':sessions[1]}
 activations=[]
 def windows_run(capture):
  for index,session in enumerate(sessions):
   if index:
    end=time.monotonic()+5
    while True:
     raw=(environment.session/'ap1-client.jsonl').read_bytes()
     if len(raw)>192*1024:raise RuntimeError('AP2 host report bound')
     closed=[json.loads(line) for line in raw.splitlines() if line.endswith(b'}')]
     closed=[r for r in closed if r.get('event')=='ap2_host_closed']
     if closed:
      if len(closed)!=1 or closed[0].get('case')!='positive' or closed[0].get('terminate_result')!=0 or closed[0].get('module_unloaded') is not True:raise RuntimeError('first native unload not proved')
      break
     if time.monotonic()>end:raise RuntimeError('first native unload acknowledgement missing')
     time.sleep(.01)
    # Preserve owned first-session mapping/configuration until the entire batch
    # checkpoint is retained. The proxy has closed and unmapped before this gate.
    first=environment.session/'ap2-first-closed';first.mkdir()
    for name in ('ap1.audio','ap1.control'):(environment.session/name).rename(first/name)
    (environment.session/'ap2.reopen').write_text('first Windows endpoint contained\n')
    end=time.monotonic()+5
    while not (environment.session/'ap1.control').exists():
     if time.monotonic()>end:raise RuntimeError('fresh proxy endpoint setup timeout')
     time.sleep(.01)
   observed=diagnostic.supervise(environment,mode=mode,checkpoint=capture,profile=profile,session_override=session)
   activations.append(observed)
   capture("ap2_activation_retained",{"records":[],"classification":observed["classification"],"cleanup":observed["cleanup"],"activations":activations})
   if observed['classification']!='scanner_completed' or observed['cleanup']!={'owned_descendants_zero':True,'process_group_empty':True}:
    return {'records':[],'classification':observed['classification'],'cleanup':observed['cleanup'],'activations':activations}
  return {'records':[],'classification':observed['classification'],'cleanup':observed['cleanup'],'activations':activations}
 return companion.supervise(environment,mode=mode,checkpoint=checkpoint,profile=profile,
  caller_command=[str(_native/'ap2-offline-host'),str(_native/'AGainOfflineBridge.vst3'),'batch'],caller_env=env,
  windows_run=windows_run,session=sessions[0],accepted_events={'ap2_host_ready','ap2_host_block','ap2_host_flush','ap2_host_rejections','ap2_host_compared','ap2_host_closed','ap2_host_error'})

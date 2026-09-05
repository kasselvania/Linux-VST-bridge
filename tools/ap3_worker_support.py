"""AP3 mode on the existing companion/supervisor and retained result path."""
import pathlib,os,secrets
import supervise as inherited
import ap1_worker_support as companion
from ap0_worker_support import StreamState
from ap1_runtime import verify_runtime
from ap0_artifacts import verify_host_store
from ap2_native_artifact import verify_native
from ap3_contract import MODE,normalize,validate_summary
_native=None

def verify_host(root,identity):
 return verify_host_store(root,identity,expected_branch='codex/ap3-sustained-audio-and-remote-desktop',expected_input_count=22)

def bind_client(binding):
 global _native
 from common import real_home
 root=real_home()/'.local/share/linux-vst-bridge/native-artifacts/by-manifest'/binding['manifest_sha256']
 verify_native(root,binding,True)
 if not os.access(root/'ap3-sustained-host',os.X_OK):raise RuntimeError('AP3 SDK host is not executable')
 for path in pathlib.Path('/proc').glob('[0-9]*/cmdline'):
  try:raw=path.read_bytes()[:16384]
  except (FileNotFoundError,PermissionError,ProcessLookupError):continue
  if b'ap3-sustained-host' in raw:raise RuntimeError('prior AP3 caller exists')
 _native=root

def command_vector(environment,session,component_case,mode):
 if mode!=MODE:raise RuntimeError('AP3 mode differs')
 vector=inherited.command_vector(environment,session,component_case,inherited.PC0_MODE)
 vector[vector.index('--mode')+1]=MODE;return vector

def supervise(environment,*,mode,checkpoint,profile):
 if _native is None:raise RuntimeError('AP3 artifact was not admitted')
 session=secrets.token_hex(16)
 env={**inherited.controlled_environment(environment),'LVB_AP2_SESSION_DIR':str(environment.session),'LVB_AP2_SESSION':session,'LD_PRELOAD':str(_native/'libap3-callback-audit.so')}
 return companion.supervise(environment,mode=mode,checkpoint=checkpoint,profile=profile,session=session,
  caller_command=[str(_native/'ap3-sustained-host'),str(_native/'AGainQueuedBridge.vst3'),'core'],caller_env=env,
  accepted_events={'ap3_host_ready','ap3_host_compared','ap3_host_closed','ap3_host_error','ap3_proxy_stats'})

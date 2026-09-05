"""AP4 composes the existing serial companion, GUI launch and retention paths."""
import pathlib,os,secrets,json,hashlib
import supervise as inherited
import ap1_worker_support as companion
import ap3_worker_support as previous
from ap0_worker_support import StreamState
from ap1_runtime import verify_runtime
from ap0_artifacts import verify_host_store
from ap2_native_artifact import verify_native
from ap4_contract import MODE,SDK,GUI,CLEAN,normalize,normalize_session,validate_summary
from common import real_home
_native=None
_gui=False

def configure(gui):
 global _gui
 if type(gui)is not bool:raise RuntimeError('AP4 bounded batch selection absent')
 _gui=gui

def gui_root():return real_home()/'AP4-State-Test'
def project_path():return gui_root()/'AP4-Test/AP4-Test.bwproject'
def verify_host(root,identity):return verify_host_store(root,identity,expected_branch='codex/ap4-plugin-state-project-recall',expected_input_count=23)
def bind_client(binding):
 global _native
 root=real_home()/'.local/share/linux-vst-bridge/native-artifacts/by-manifest'/binding['manifest_sha256'];verify_native(root,binding,True,True)
 if not os.access(root/'ap3-sustained-host',os.X_OK):raise RuntimeError('AP4 SDK host unavailable')
 for p in pathlib.Path('/proc').glob('[0-9]*/cmdline'):
  try:raw=p.read_bytes()[:16384]
  except (FileNotFoundError,PermissionError,ProcessLookupError):continue
  if b'ap3-sustained-host' in raw:raise RuntimeError('prior state caller exists')
 _native=root
 if _gui:
  project_digest()
  relative='AGainQueuedBridge.vst3/Contents/x86_64-linux/AGainQueuedBridge.so'
  p=gui_root()/'plugins'/relative
  if p.is_symlink() or not p.is_file() or p.read_bytes()!=(root/relative).read_bytes():raise RuntimeError('AP4 temporary publication differs')

def command_vector(environment,session,component_case,mode):
 if mode!=MODE:raise RuntimeError('AP4 mode differs')
 v=inherited.command_vector(environment,session,component_case,inherited.PC0_MODE);v[v.index('--mode')+1]=MODE;return v

def progress(environment,phase):
 root=gui_root();root.mkdir(mode=0o700,exist_ok=True)
 inherited.write_atomic(root/'live.json',(json.dumps(dict(run_id=environment.run_id,session_dir=str(environment.session),phase=phase))+'\n').encode())

def core(environment,*,mode,checkpoint,profile,label):
 if _native is None:raise RuntimeError('AP4 native artifact not admitted')
 store=environment.session/'ap4-state-store';store.mkdir(mode=0o700,exist_ok=True)
 session=secrets.token_hex(16)
 env={**inherited.controlled_environment(environment),'LVB_AP2_SESSION_DIR':str(environment.session),'LVB_AP2_SESSION':session,'LVB_AP4_STATE_STORE':str(store),'LVB_AP4_COMPARE':'1','LD_PRELOAD':str(_native/'libap3-callback-audit.so')}
 return companion.supervise(environment,mode=mode,checkpoint=checkpoint,profile=profile,session=session,
  caller_command=[str(_native/'ap3-sustained-host'),str(_native/'AGainQueuedBridge.vst3'),label.replace('_','-')],caller_env=env,
  accepted_events={'ap4_host_state','ap4_host_compared','ap4_native_state','ap4_native_error','ap4_sample_comparison','ap3_host_closed','ap3_host_error','ap3_proxy_stats'})

def project_digest():
 p=project_path()
 if p.is_symlink() or not p.is_file() or not 0<p.stat().st_size<=8*1024*1024:raise RuntimeError('AP4 saved test project unavailable')
 return hashlib.sha256(p.read_bytes()).hexdigest()

def gui(environment,*,mode,checkpoint,profile,label,prior):
 before=project_digest();expected=prior if prior is not None else before
 if before!=expected:raise RuntimeError('saved project changed before reopen')
 result=previous.gui(environment,mode=mode,checkpoint=checkpoint,profile=profile,label=label,
  root=gui_root(),project=project_path(),native=_native,extra_env={'LVB_AP4_COMPARE':'1'},
  accepted_events={'ap4_bitwig_ui','ap4_native_state','ap4_native_error','ap4_sample_comparison','ap3_proxy_stats','ap3_proxy_lifecycle'})
 result['caller']['records'].append(dict(event='ap4_project',case=label,project_before_sha256=before,expected_before_sha256=expected,project_sha256=project_digest()))
 return result

def supervise(environment,*,mode,checkpoint,profile):
 segments={};previous_project=None
 for label in SDK+(GUI if _gui else ()):
  progress(environment,label);outer=checkpoint
  def capture(stage,available=None,error=None):
   cleanup=available.get('cleanup',{}) if available else {}
   outer(stage,{'segments':segments,'current':available,'cleanup':{k:cleanup.get(k)is True for k in CLEAN}} if available else None,error)
  observed=(core(environment,mode=mode,checkpoint=capture,profile=profile,label=label) if label in SDK else gui(environment,mode=mode,checkpoint=capture,profile=profile,label=label,prior=previous_project))
  segments[label]=observed;checkpoint('ap4_segment_retained',{'segments':segments,'cleanup':observed['cleanup'],'classification':observed['classification']})
  if observed['cleanup']!=CLEAN or observed['classification']!='scanner_completed':return dict(segments=segments,cleanup=observed['cleanup'],classification=observed['classification'])
  # No next plug-in instance after a reporting/admission failure.
  normalize_session(observed,label)
  if label in GUI:previous_project=project_digest()
  closed=environment.session/('ap4-'+label+'-closed');closed.mkdir()
  for name in ('ap1.audio','ap1.control','ap1-client.jsonl','ap1-client.stderr','ap3-gui-report.jsonl'):
   p=environment.session/name
   if p.exists():p.rename(closed/name)
 progress(environment,'batch_completed')
 return dict(segments=segments,cleanup=CLEAN,classification='scanner_completed')

"""AP3 mode on the existing companion/supervisor and retained result path."""
import pathlib,os,secrets,json,time,subprocess,hashlib
import supervise as inherited
import ap1_worker_support as companion
from ap0_worker_support import StreamState
from ap1_runtime import verify_runtime
from ap0_artifacts import verify_host_store
from ap2_native_artifact import verify_native
from ap3_contract import MODE,normalize,normalize_session,validate_summary
from common import real_home
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
 verify_gui_inputs()

def command_vector(environment,session,component_case,mode):
 if mode!=MODE:raise RuntimeError('AP3 mode differs')
 vector=inherited.command_vector(environment,session,component_case,inherited.PC0_MODE)
 vector[vector.index('--mode')+1]=MODE;return vector

def core(environment,*,mode,checkpoint,profile,case="core"):
 if _native is None:raise RuntimeError('AP3 artifact was not admitted')
 session=secrets.token_hex(16)
 env={**inherited.controlled_environment(environment),'LVB_AP2_SESSION_DIR':str(environment.session),'LVB_AP2_SESSION':session,'LD_PRELOAD':str(_native/'libap3-callback-audit.so')}
 return companion.supervise(environment,mode=mode,checkpoint=checkpoint,profile=profile,session=session,
  caller_command=[str(_native/'ap3-sustained-host'),str(_native/'AGainQueuedBridge.vst3'),case],caller_env=env,
  accepted_events={'ap3_host_ready','ap3_host_compared','ap3_host_closed','ap3_host_error','ap3_proxy_stats'})


def gui_root():return real_home()/'AP3-Preview-Test'

GUI_INPUTS={'AP3-Test/AP3-Test.bwproject':'9cec6ccc55e19457fba6cf018100b175b94a9fe9f8975c9e01e3a8c51af50a00','AP3-low-level-stereo.wav':'9d860bcae28798a0b003d4626d505ce4fe523acc532f5ff978da3cef2c8f544e'}
def verify_gui_inputs():
 root=gui_root()
 if root.is_symlink() or not root.is_dir():raise RuntimeError('AP3 test directory unavailable')
 for relative,digest in GUI_INPUTS.items():
  p=root/relative
  if p.is_symlink() or not p.is_file() or p.stat().st_size>8*1024*1024 or hashlib.sha256(p.read_bytes()).hexdigest()!=digest:raise RuntimeError('AP3 disposable project/audio differs: '+relative)
 relative='AGainQueuedBridge.vst3/Contents/x86_64-linux/AGainQueuedBridge.so'
 p=root/'plugins'/relative
 if p.is_symlink() or not p.is_file() or hashlib.sha256(p.read_bytes()).digest()!=hashlib.sha256((_native/relative).read_bytes()).digest():raise RuntimeError('AP3 test publication differs from admitted artifact')

def progress(environment,phase):
 value=dict(run_id=environment.run_id,session_dir=str(environment.session),phase=phase)
 inherited.write_atomic(gui_root()/'live.json',(json.dumps(value)+'\n').encode())

def await_stream(environment):
 progress(environment,'await_stream_active')
 gate=environment.session/'stream-active.json';end=time.monotonic()+120
 while not gate.exists():
  if time.monotonic()>end:raise RuntimeError('Moonlight stream-active confirmation timeout')
  time.sleep(.1)
 if gate.is_symlink() or gate.stat().st_size>1024:raise RuntimeError('unsafe stream confirmation')
 if json.loads(gate.read_bytes())!={'run_id':environment.run_id,'stream_active':True}:raise RuntimeError('stream confirmation binding differs')

def gui(environment,*,mode,checkpoint,profile,label,root=None,project=None,native=None,accepted_events=None,extra_env=None):
 root=gui_root() if root is None else root;project=root/'AP3-Test/AP3-Test.bwproject' if project is None else project
 native=_native if native is None else native
 if not project.is_file():raise RuntimeError('prepared disposable Bitwig project missing')
 # This one test publication must match the admitted retained native artifact.
 relative='AGainQueuedBridge.vst3/Contents/x86_64-linux/AGainQueuedBridge.so'
 if hashlib.sha256((root/'plugins'/relative).read_bytes()).digest()!=hashlib.sha256((native/relative).read_bytes()).digest():raise RuntimeError('Bitwig publication differs from native artifact')
 if subprocess.check_output(['flatpak','info','--show-commit','com.bitwig.BitwigStudio'],text=True).strip()!='8a048e733e74dda8b897339436153a2d5df952f29d362dfcaca9d8e0d6f6c231':raise RuntimeError('Bitwig installation changed')
 session=secrets.token_hex(16);report=environment.session/'ap3-gui-report.jsonl'
 # Import only the session-display keys. Never export the service environment.
 manager=subprocess.check_output(['systemctl','--user','show-environment'],text=True)
 display={}
 for line in manager.splitlines():
  k,sep,v=line.partition('=')
  if sep and k in {'DISPLAY','WAYLAND_DISPLAY','XAUTHORITY','DBUS_SESSION_BUS_ADDRESS','XDG_SESSION_TYPE'}:display[k]=v
 env={**inherited.controlled_environment(environment),**display}
 for k in ('XDG_CACHE_HOME','XDG_CONFIG_HOME','XDG_DATA_HOME','TMPDIR'):env.pop(k,None)
 command=['flatpak','run','--filesystem='+str(environment.session),
  '--env=LVB_AP2_SESSION_DIR='+str(environment.session),'--env=LVB_AP2_SESSION='+session,'--env=LVB_AP3_REPORT='+str(report),
  '--nofilesystem='+str(real_home()/'.vst3/yabridge'),'--nofilesystem='+str(real_home()/'.vst3/VCV Rack 2'),
  '--nofilesystem='+str(real_home()/'.vst'),'--nofilesystem='+str(real_home()/'.clap'),
  *['--env='+k+'='+v for k,v in (extra_env or {}).items()],
  'com.bitwig.BitwigStudio',str(project)]
 def retained_report():
  if report.is_symlink() or not report.is_file() or report.stat().st_size>8192:raise RuntimeError('bounded native Bitwig report missing')
  raw=report.read_bytes()
  # The UI note is an agent observation, never the numerical/timing oracle.
  note=environment.session/(label+'-ui.json')
  if note.is_file() and not note.is_symlink() and note.stat().st_size<=4096:
   value=json.loads(note.read_bytes())
   if value.get('run_id')!=environment.run_id or value.get('case')!=label:raise RuntimeError('Bitwig UI note binding differs')
   raw+=(json.dumps(value)+'\n').encode()
  return raw
 progress(environment,label)
 return companion.supervise(environment,mode=mode,checkpoint=checkpoint,profile=profile,session=session,
  caller_command=command,caller_env=env,ready_seconds=180,exit_seconds=60,caller_report=retained_report,track_descendants=True,
  accepted_events=accepted_events or {'ap3_proxy_stats','ap3_proxy_lifecycle','ap3_bitwig_ui'})

def supervise(environment,*,mode,checkpoint,profile):
 segments={};clean={'owned_descendants_zero':True,'process_group_empty':True}
 for label,run in [('core',lambda:core(environment,mode=mode,checkpoint=checkpoint,profile=profile)),
                   ('stream_active',lambda:core(environment,mode=mode,checkpoint=checkpoint,profile=profile,case='reopen')),
                   ('bitwig_first',lambda:gui(environment,mode=mode,checkpoint=checkpoint,profile=profile,label='bitwig_first')),
                   ('bitwig_reopen',lambda:gui(environment,mode=mode,checkpoint=checkpoint,profile=profile,label='bitwig_reopen'))]:
  if label=='stream_active':await_stream(environment)
  progress(environment,label)
  outer_checkpoint=checkpoint
  def capture(stage,available=None,error=None):
   # Completed segments were admitted and contained before continuing.
   # The companion's final checkpoint covers BOTH its caller and Windows.
   cleanup=available.get('cleanup',{}) if available else {}
   outer_checkpoint(stage,{'segments':segments,'current':available,'cleanup':{k:cleanup.get(k) is True for k in clean}} if available else None,error)
  # Existing companion and Windows supervisor own every process and timeout.
  original=checkpoint;checkpoint=capture
  try:observed=run()
  finally:checkpoint=original
  segments[label]=observed
  checkpoint('ap3_segment_retained',{'segments':segments,'cleanup':observed['cleanup'],'classification':observed['classification']})
  if observed['cleanup']!=clean or observed['classification']!='scanner_completed':
   return dict(segments=segments,cleanup=observed['cleanup'],classification=observed['classification'])
  # Refuse another live segment when the retained caller/report is invalid.
  normalize_session(observed,label)
  closed=environment.session/('ap3-'+label+'-closed');closed.mkdir()
  for name in ('ap1.audio','ap1.control','ap1-client.jsonl','ap1-client.stderr','ap3-gui-report.jsonl'):
   path=environment.session/name
   if path.exists():path.rename(closed/name)
 progress(environment,'batch_completed')
 return dict(segments=segments,cleanup=clean,classification='scanner_completed')

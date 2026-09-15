"""One human action on an exact source-owned fixture. No input methods are called."""
import ctypes as C
import json
import mmap
import os
from pathlib import Path
import signal
import struct
import sys
import time
HERE=Path(__file__).resolve().parent
sys.path[:0]=[str(HERE),str(HERE.parent/'uio3'),str(HERE.parent/'uio1'),str(HERE.parent.parent/'bridge-manager/runtime')]
from launch import Helper,sealed_bytes,private_json
from x11 import X11,P,U,I
from xi2 import XI2,XIRecorder
import ownership
from analysis import unfold32,classify,INSUFFICIENT

CAPACITY=262144;SIZE=4096+CAPACITY*128
HEADER='magic version bytes pid start tid root child frequency ready stop armed committed dropped finished error clock_request clock_ack clock_qpc clock_tick turns down up returns heartbeat destroyed calibration_ok xid child_xid'.split()
FIELDS='commit kind qpc end_qpc turn dispatch hwnd capture focus active message message_time tick queue flags pointer_id pointer_type pointer_flags x y value'.split()
RECORD=struct.Struct('<10Q8I2iQ')
class Mapping:
 def __init__(self,path):
  self.file=path.open('r+b');self.map=mmap.mmap(self.file.fileno(),SIZE);self.cursor=0;self.rows=[]
  if [self.read(x) for x in ('magic','version','bytes')]!=[0x32524955,1,SIZE]:raise RuntimeError('fixture status schema')
  self.identity=tuple(self.read(x) for x in ('pid','start','tid','root','child','frequency'))
 def read(self,key):return struct.unpack_from('<Q',self.map,HEADER.index(key)*8)[0]
 def write(self,key,n):
  if key not in ('stop','armed','clock_request'):raise ValueError('closed fixture mailbox')
  struct.pack_into('<Q',self.map,HEADER.index(key)*8,n)
 def take(self):
  if tuple(self.read(x) for x in ('pid','start','tid','root','child','frequency'))!=self.identity:raise RuntimeError('fixture identity changed')
  count=self.read('committed')
  if not self.cursor<=count<=CAPACITY:raise RuntimeError('fixture record capacity/counter')
  for n in range(self.cursor,count):
   r=dict(zip(FIELDS,RECORD.unpack_from(self.map,4096+n*128)))
   if r['commit']!=n+1 or not 1<=r['kind']<=6:raise RuntimeError('fixture torn/unknown record')
   self.rows.append(r)
  self.cursor=count
 def clock(self,helper):
  n=self.read('clock_request')+1;before=time.monotonic_ns();self.write('clock_request',n);deadline=time.monotonic()+1
  while self.read('clock_ack')!=n:
   helper.poll()
   if time.monotonic()>deadline:raise RuntimeError('fixture clock response unavailable')
   time.sleep(.002)
  return dict(linux_before_ns=before,linux_after_ns=time.monotonic_ns(),windows_qpc=self.read('clock_qpc'),
              tick=self.read('clock_tick'),frequency=self.read('frequency'))
 def close(self):self.map.close();self.file.close()

class PropertyEvent(C.Structure):
 _fields_=[('type',I),('serial',U),('send_event',I),('display',P),('window',U),('atom',U),('time',U),('state',I)]

def xclock(x,ordinal):
 """Source-owned PropertyNotify brackets X server time without any input."""
 lib=x.x;lib.XSelectInput.argtypes=[P,U,C.c_long];lib.XSelectInput(x.display,x.window,1<<22)
 lib.XChangeProperty.argtypes=[P,U,U,U,I,I,P,I];lib.XPending.argtypes=[P];lib.XPending.restype=I
 lib.XNextEvent.argtypes=[P,P];lib.XFlush.argtypes=[P]
 atom=lib.XInternAtom(x.display,b'LVB_UIR2_CLOCK',0);data=C.c_ubyte(ordinal%256)
 before=time.monotonic_ns();lib.XChangeProperty(x.display,x.window,atom,6,8,0,C.byref(data),1);lib.XFlush(x.display)
 deadline=time.monotonic()+1
 while time.monotonic()<deadline:
  if lib.XPending(x.display):
   buf=(C.c_long*24)();lib.XNextEvent(x.display,C.byref(buf));event=C.cast(C.byref(buf),C.POINTER(PropertyEvent)).contents
   if event.type==28 and event.window==x.window and event.atom==atom:
    return dict(server_ms=int(event.time)&0xffffffff,linux_before_ns=before,linux_after_ns=time.monotonic_ns())
  time.sleep(.001)
 raise RuntimeError('X server clock unavailable')

def finish(h):
 r=h.finish();c=r.get('cleanup',{})
 if r['exit']!=0 or r['overflow'] or c.get('owned_descendants_zero') is not True or c.get('process_group_empty') is not True:
  raise RuntimeError('fixture positive cleanup unavailable')
 return r

def make_environment(root,desktop):
 if not desktop.get('DISPLAY') or not desktop.get('XAUTHORITY'):raise RuntimeError('physical XWayland desktop unavailable')
 env={k:desktop[k] for k in ('DISPLAY','XAUTHORITY')}
 env.update(PATH='/usr/bin:/bin',LANG='C.UTF-8',STEAM_COMPAT_APP_ID='0',SteamAppId='0',SteamGameId='0',STEAM_ZENITY='',WINEDEBUG='-all')
 for key,name in [('HOME','home'),('XDG_CONFIG_HOME','config'),('XDG_CACHE_HOME','cache'),('XDG_DATA_HOME','data'),
   ('STEAM_COMPAT_DATA_PATH','compatdata'),('STEAM_COMPAT_CLIENT_INSTALL_PATH','client'),('PRESSURE_VESSEL_VARIABLE_DIR','runtime-var'),('TMPDIR','tmp')]:
  d=root/name;d.mkdir(mode=0o700);env[key]=str(d)
 # Physical display sockets are intentionally shared; no private compositor is created.
 if desktop.get('XDG_RUNTIME_DIR'):env['XDG_RUNTIME_DIR']=desktop['XDG_RUNTIME_DIR']
 return env

def summary(raw):
 from timeline import projected
 start=raw.get('action_begin_ns',0)
 contacts=[r for r in raw.get('raw',[]) if r['kind']=='raw_touch_begin' and r['observed_ns']>=start]
 ends=[r for r in raw.get('raw',[]) if r['kind']=='raw_touch_end' and r['observed_ns']>=start]
 core=[r for r in raw.get('x11',[]) if r['kind']=='core_up' and r['observed_ns']>=start]
 removals=[r for r in raw.get('windows',[]) if r['kind']==2 and r['value']==1 and r['message'] in (0x247,0x202)]
 procedures=[r for r in raw.get('windows',[]) if r['kind']==4 and r['message'] in (0x247,0x202) and r['dispatch']]
 clocks=raw.get('windows_clocks',[]);xclocks=raw.get('x_clocks',[])
 # Raw contacts are private source scoped. A single exact source/detail must
 # have one begin/end; matching counts alone are insufficient.
 exact=(len(contacts)==len(ends)==1 and all(contacts[0][k]==ends[0][k] for k in ('device','source','detail')))
 facts=dict(contacts=len(contacts),coverage_complete=exact and len(core)==1 and not any(raw.get('drops',{}).values()) and raw.get('error') is None,
   clock_validated=bool(len(clocks)>=2 and len(xclocks)>=2 and raw.get('status',{}).get('calibration_ok',0)),procedure_after_removal=False)
 delays=[];gaps=[];message_times=[]
 if len(core)==1 and removals:
  event=core[0];b=min(xclocks,key=lambda b:abs(((event['server_ms']-b['server_ms']+2**31)%2**32)-2**31)) if xclocks else None
  if b:
   d=(unfold32(event['server_ms'],b['server_ms'])-b['server_ms'])*1000000
   xlower=b['linux_before_ns']+d-abs(d)//10000-1000000
   xupper=b['linux_after_ns']+d+abs(d)//10000+1000000
   for r in removals:
    i=projected(r['end_qpc'],clocks)
    if i:delays.append(((i[0]-xupper)/1e6,(i[1]-xlower)/1e6))
    bwin=min(clocks,key=lambda c:abs(r['end_qpc']-c['windows_qpc']))
    dt=(unfold32(r['message_time'],bwin['tick'])-bwin['tick'])*1000000
    # Validated source-owned posted sentinel bounds tick rounding; no claim of
    # Wine's internal creation moment follows from the inherited MSG.time.
    message_times.append([(bwin['linux_before_ns']+dt-64000000-xupper)/1e6,
                          (bwin['linux_after_ns']+dt+64000000-xlower)/1e6])
   # Use actual dispatch identity, not sent-call hooks or ordinal contact pairing.
   dispatches={r['dispatch']:r for r in raw['windows'] if r['kind']==3}
   for p in procedures:
    drow=dispatches.get(p['dispatch']);matches=[r for r in removals if r['message']==p['message'] and r['hwnd']==p['hwnd'] and drow and r['end_qpc']<=drow['qpc']]
    if matches:
     r=max(matches,key=lambda r:r['end_qpc']);gaps.append((p['qpc']-r['end_qpc'])*1000/raw['status']['frequency'])
   facts.update(procedure_after_removal=bool(gaps) and len(gaps)==len(procedures),
     removal_lower_ms=min((d[0] for d in delays),default=0),removal_upper_ms=max((d[1] for d in delays),default=float('inf')),
     procedure_gap_upper_ms=max(gaps,default=float('inf')))
 # No queue flag is promoted to exact availability/absence or creation proof.
 disposition=classify(facts)
 clean=raw.get('cleanup',{}).get('fixture',{}).get('cleanup',{})
 return dict(schema=1,disposition=disposition,contacts=len(contacts),core_releases=len(core),windows_release_removals=len(removals),
    removal_intervals_ms=delays,message_timestamp_intervals_after_x_release_ms=message_times,procedure_gaps_ms=gaps,queue_bits_are_hints=True,creation_time_not_proven=True,
    dropped=raw.get('drops',{}),cleanup=clean,clock_calibrated=facts['clock_validated'],error_present=raw.get('error') is not None)

def run(root,package,runner_file,self_test=False):
 os.umask(0o077);root.mkdir(mode=0o700)
 if root.resolve()!=root:raise RuntimeError('scratch path alias')
 runner=json.loads(runner_file.read_text())
 if Path(runner['entry_point']).stat().st_dev!=root.stat().st_dev:raise RuntimeError('scratch filesystem differs from pinned runtime')
 import hashlib,subprocess
 for a in runner['files']:
  with open(a['path'],'rb') as f:
   if hashlib.file_digest(f,'sha256').hexdigest()!=a['sha256']:raise RuntimeError('runner artifact changed')
 manifest=json.loads((HERE/'package.json').read_text());payload=sealed_bytes(package,manifest['sha256'])
 desktop=dict(line.split('=',1) for line in subprocess.check_output(['systemctl','--user','show-environment'],text=True).splitlines() if '=' in line)
 env=make_environment(root,desktop);os.environ.update({k:env[k] for k in ('DISPLAY','XAUTHORITY')})
 base=[runner['entry_point'],'--verb=run','--',runner['proton']]
 raw=dict(schema=1,windows=[],windows_clocks=[],x_clocks=[],raw=[],x11=[],drops={},cleanup={},error=None,manifest=manifest)
 fixture=None;m=x=xi=rec=None;stop=False
 def cancel(*_):
  nonlocal stop;stop=True
 signal.signal(signal.SIGTERM,cancel);signal.signal(signal.SIGINT,cancel)
 try:
  target=root/'compatdata/pfx/drive_c/uir2';target.mkdir(parents=True,mode=0o700)
  exe=target/'fixture.exe';exe.write_bytes(payload);exe.chmod(0o500)
  raw['cleanup']['initialize']=finish(Helper(ownership,[*base,'getcompatpath',str(target)],env,target,root/'initialize.log',60))
  if self_test:
   raw['cleanup']['fixture']=finish(Helper(ownership,[*base,'runinprefix','C:\\uir2\\fixture.exe','--self-test'],env,target,root/'self-test.log',20))
   return
  status=target/'status.bin';fixture=Helper(ownership,[*base,'runinprefix','C:\\uir2\\fixture.exe','C:\\uir2\\status.bin'],env,target,root/'fixture.log',175)
  deadline=time.monotonic()+15
  while not status.exists() or status.stat().st_size!=SIZE:
   fixture.poll()
   if time.monotonic()>deadline:raise RuntimeError('fixture status unavailable')
   time.sleep(.01)
  while True:
   with status.open('rb') as f:initial=f.read(80)
   if len(initial)==80 and struct.unpack_from('<Q',initial,72)[0]==1:break
   fixture.poll()
   if time.monotonic()>deadline:raise RuntimeError('fixture header not complete')
   time.sleep(.01)
  m=Mapping(status)
  while not m.read('ready'):
   fixture.poll()
   if time.monotonic()>deadline:raise RuntimeError('fixture ready unavailable')
   time.sleep(.01)
  xid=m.read('xid')
  if not xid:raise RuntimeError('exact Wine XWayland binding unavailable')
  x=X11(xid);pid=x.property(xid,'_NET_WM_PID')
  if len(pid)!=1:raise RuntimeError('exact X11 process unavailable')
  x.pid=pid[0];x.check_identity();raw['identity']={k:m.read(k) for k in ('pid','start','tid','root','child','xid','child_xid')}
  targets={xid};targets.update(v for v in (m.read('child_xid'),) if v)
  xi=XI2(x,targets);rec=XIRecorder(x,xi.opcode,xi.devices,targets);xi.action=rec.action=1
  # One fixed begin label only. There is no repeat, input or coordinate mailbox.
  private_json(root/'ready.json',dict(schema=1,ready=True,input_injection=False,maximum_contacts=1))
  next_clock=0;deadline=time.monotonic()+135;armed=False;released=None
  while not stop and time.monotonic()<deadline:
   fixture.poll();m.take()
   if m.read('error') or m.read('dropped'):raise RuntimeError('fixture failed or overflowed')
   now=time.monotonic()
   if now>=next_clock:
    raw['windows_clocks'].append(m.clock(fixture));raw['x_clocks'].append(xclock(x,len(raw['x_clocks'])+1));next_clock=now+2
   if not armed and (root/'begin.json').exists():
    p=root/'begin.json'
    if p.is_symlink() or p.stat().st_size>16 or json.loads(p.read_text())!=1:raise RuntimeError('action label differs')
    if any(r['kind']=='raw_touch_begin' for r in xi.records):raise RuntimeError('touch_before_action_label')
    m.write('armed',1);armed=True;raw['action_begin_ns']=time.monotonic_ns()
   xi.poll();rec.devices=set(xi.devices);rec.poll();x.check_identity()
   if armed:
    begins=[r for r in xi.records if r['kind']=='raw_touch_begin' and r['observed_ns']>=raw['action_begin_ns']]
    ends=[r for r in xi.records if r['kind']=='raw_touch_end' and r['observed_ns']>=raw['action_begin_ns']]
    if len(begins)>1:raw['error']='multiple_physical_contacts';break
    if ends and released is None:released=now
    if released is not None and now-released>20:break
   time.sleep(.005)
  if not armed:raw['error']='physical_action_not_armed'
  raw['final_pointer']=x.pointer()
 except BaseException as error:
  raw['error']=type(error).__name__
  raise
 finally:
  if rec:
   rec.close();raw['x11']=rec.records;raw['drops']['xrecord']=rec.dropped
  if xi:
   raw['raw']=xi.records;raw['drops']['xi2']=xi.dropped;raw['devices']=xi.device_history;xi.close()
  if x:x.close()
  if m:m.write('stop',1)
  if fixture:
   try:raw['cleanup']['fixture']=finish(fixture)
   except Exception as e:raw['cleanup']['error']=type(e).__name__;raw['error']='fixture_cleanup_incomplete'
  if m:
   m.take();raw['windows']=m.rows;raw['status']={k:m.read(k) for k in HEADER};raw['drops']['windows']=m.read('dropped')
   if not m.read('finished') or not m.read('destroyed') or m.read('error'):raw['error']='fixture_retirement_incomplete'
   m.close()
  private_json(root/'timeline.json',raw)
  if not self_test:private_json(root/'summary.json',summary(raw))

if __name__=='__main__':
 try:run(*map(Path,sys.argv[1:4]),self_test=sys.argv[4:]==['--self-test'])
 except Exception as e:print(json.dumps({'completed':False,'error_class':type(e).__name__}));raise
 else:print(json.dumps({'completed':True}))

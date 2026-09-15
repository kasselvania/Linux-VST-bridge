#!/usr/bin/env python3
"""Private source-owned proof through the actual --install supervisor and cgroup.
No installed software mutation, vendor payload, input injection or account access.
"""
import argparse,hashlib,json,os,pathlib,subprocess,time
p=argparse.ArgumentParser();p.add_argument('--runtime',type=pathlib.Path,required=True);p.add_argument('--payload',type=pathlib.Path,required=True);p.add_argument('--output',type=pathlib.Path,required=True);p.add_argument('--cancel-ready',action='store_true');a=p.parse_args()
os.umask(0o077);a.output.mkdir(mode=0o700)
h=pathlib.Path.home();m=h/'.local/share/linux-vst-bridge/managed';cli=h/'.local/bin/linux-vst-bridge'
cap=json.loads(subprocess.check_output([str(cli),'capacity']))['capacity'];assert cap['dsp']==0 and cap['maintenance']==0 and not cap['cleanup_unconfirmed']
# Reuse only the pinned runner identity, never the Arturia environment itself.
runners=json.loads((m/'registry.json').read_text())['classes'];runner=next(iter(runners.values()))['registration']['environment']['runner']
op=os.urandom(16).hex();root=a.output/'environment';root.mkdir(mode=0o700)
for name in ['home','compatdata','runtime-var','host-cache','host-config','host-data','host-tmp','client']:(root/name).mkdir(mode=0o700)
(root/'operation.lock').touch(mode=0o600)
artifact={'path':str(a.payload.resolve(strict=True)),'sha256':hashlib.sha256(a.payload.read_bytes()).hexdigest()}
spec={'schema':2,'operation':op,'environment':{'id':op,'root':str(root.resolve()),'revision':1,'runner':runner},'installer':artifact,'format':'pe_executable','report':str((a.output/'result.json').resolve())}
(a.output/'sources.json').write_text(json.dumps({name:hashlib.sha256(path.read_bytes()).hexdigest() for name,path in [('session.py',a.runtime/'session.py'),('ownership.py',a.runtime/'ownership.py'),('bootstrap_fixture.py',pathlib.Path(__file__)),('payload',a.payload)]},indent=2))
sp=a.output/'spec.json';sp.write_text(json.dumps(spec));unit='linux-vst-bridge-installer-'+op+'.service'
started=time.monotonic();subprocess.run(['systemd-run','--user','--collect','--property=UMask=0077','--property=KillMode=control-group','--property=TimeoutStopSec=20','--property=StandardOutput=null','--property=StandardError=null','--unit='+unit,'/usr/bin/python3',str(a.runtime.resolve()/'session.py'),'--install',str(sp.resolve())],check=True)
reason='deadline';snap=None;identities=[]
try:
 while time.monotonic()-started<90:
  rp=a.output/'result.json'
  if rp.exists():
   try:snap=json.loads(rp.read_text())
   except json.JSONDecodeError:continue
   startup=snap.get('startup',{})
   if startup.get('first_problem'):reason='startup_problem';break
   if a.cancel_ready and startup.get('target_observation')=='observed':reason='target_observed_cancel';break
   if snap.get('cleanup_confirmed'):reason='retired';break
  time.sleep(.1)
 # Exact unit's process identities and limited container-visible launch contract.
 cg=subprocess.check_output(['systemctl','--user','show',unit,'-p','ControlGroup','--value'],text=True).strip()
 if cg:
  for cp in (pathlib.Path('/sys/fs/cgroup')/cg.lstrip('/')).rglob('cgroup.procs'):
   for pid in cp.read_text().split():
    try:
     q=pathlib.Path('/proc')/pid;st=(q/'stat').read_text();env=dict(x.split(b'=',1) for x in (q/'environ').read_bytes().split(b'\0') if b'=' in x)
     values={k.decode():v.decode(errors='replace') for k,v in env.items() if k in [b'HOME',b'WINEPREFIX',b'STEAM_COMPAT_CLIENT_INSTALL_PATH',b'LD_LIBRARY_PATH',b'WINEDEBUG']}
     candidate=q/'root'/str(root.resolve()/'home/.steam/sdk64/steamclient.so').lstrip('/')
     identities.append({'pid':int(pid),'start_ticks':int(st.rsplit(')',1)[1].split()[19]),'comm':(q/'comm').read_text().strip(),'cmdline':(q/'cmdline').read_bytes().replace(b'\0',b' ').decode(errors='replace'),'environment':values,'private_home_steamclient_exists':candidate.exists()})
    except (FileNotFoundError,PermissionError,ProcessLookupError):pass
 (a.output/'before-cleanup.json').write_text(json.dumps({'reason':reason,'result':snap,'processes':identities},indent=2))
finally:
 if subprocess.run(['systemctl','--user','is-active','--quiet',unit]).returncode==0:
  subprocess.run(['systemctl','--user','stop',unit],check=True,timeout=30)
final=json.loads((a.output/'result.json').read_text());log=a.output/(op+'-private.log');raw=log.read_bytes() if log.exists() else b''
alive=[]
for row in identities:
 try:
  st=(pathlib.Path('/proc')/str(row['pid'])/'stat').read_text()
  if int(st.rsplit(')',1)[1].split()[19])==row['start_ticks']:alive.append(row['pid'])
 except FileNotFoundError:pass
proof={'schema':1,'operation':op,'payload_sha256':artifact['sha256'],'production_supervisor':True,'fresh_private_home':True,'reason':reason,'ready_marker':b'MF2_BOOTSTRAP_READY_V1' in raw,'result':final,'same_identity_survivors':len(alive),'unit_absent':subprocess.run(['systemctl','--user','is-active','--quiet',unit]).returncode!=0}
(a.output/'proof-private.json').write_text(json.dumps(proof,indent=2));assert final['cleanup_confirmed'] and final['owned_live']==0 and not alive
print(json.dumps({'reason':reason,'ready':proof['ready_marker'],'startup_problem':final.get('startup',{}).get('first_problem'),'state':final['state'],'raw_exit':final['raw_exit'],'cleanup':True}))

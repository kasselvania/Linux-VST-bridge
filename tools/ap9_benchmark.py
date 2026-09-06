#!/usr/bin/env python3
"""Run one bounded AP9 SDK comparison against already prepared fixture owners.

Writes private numerical results only. Does not install a plugin, start a DAW,
change audio-device settings, or construct an environment. See docs/AP9.md.
"""
import argparse,re
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--name',required=True)
parser.add_argument('--role',choices=['reference','serum'],required=True)
parser.add_argument('--build',required=True,help='SHA256 directory from the retained native build receipt')
parser.add_argument('--rate',type=int,choices=[44100,48000,88200,96000,192000],default=48000)
parser.add_argument('--block',type=int,choices=[32,64,128,256,512,1024],default=128)
parser.add_argument('--delay',type=int,choices=[64,128,256,512,1024,2048],default=256)
parser.add_argument('--seconds',type=int,default=12)
parser.add_argument('--voices',type=int,default=1,help='simultaneous submitted notes; plugin voice allocation is its own policy')
args=parser.parse_args()
if not re.fullmatch('[a-z0-9-]{1,64}',args.name) or not re.fullmatch('[0-9a-f]{64}',args.build):parser.error('invalid result name or build identity')
if not 2<=args.seconds<=120 or not 1<=args.voices<=48:parser.error('duration or submitted-note bound')
if args.delay<args.block:parser.error('delay must cover one host block')
CASE=vars(args)
import pathlib,subprocess,os,json,time,signal,hashlib
h=pathlib.Path.home();task=h/'AP9-Performance';role=CASE['role'];os.umask(0o077)
assert task.is_dir() and not task.is_symlink(), 'prepared private AP9 task directory required'
output=task/'runs'/CASE['name'];output.mkdir(mode=0o700)
build=h/('.cache/linux-vst-bridge/ap9-builds/'+CASE['build']+'/build');host=build/'native-vst3-proxy/ap3-sustained-host';audit=build/'native-vst3-proxy/libap3-callback-audit.so';bundle=build/('VST3/Release/'+('CommercialInstrumentBridge' if role=='serum' else 'AGainQueuedBridge')+'.vst3')
assert task.is_dir() and not task.is_symlink(), 'prepared private AP9 task directory required'
os.umask(0o077)
setting=task/'delay-frames'
fd=os.open(setting,os.O_WRONLY|os.O_CREAT|os.O_TRUNC|os.O_NOFOLLOW,0o600)
with os.fdopen(fd,'w') as setting_file: setting_file.write(str(CASE['delay'])+'\n')
results=task/role/'results';before=set(results.glob('*'))
def usage():
 text=subprocess.check_output(['systemctl','--user','show','lvb-ap9-'+role,'--property=CPUUsageNSec,MemoryCurrent'],text=True)
 result={k:int(v) for k,_,v in (line.partition('=') for line in text.splitlines()) if v.isdigit()}
 cg=subprocess.check_output(['systemctl','--user','show','lvb-ap9-'+role,'--property=ControlGroup','--value'],text=True).strip()
 processes=[]
 if cg:
  for f in (pathlib.Path('/sys/fs/cgroup')/cg.lstrip('/')).rglob('cgroup.procs'):
   for pid in f.read_text().split():
    try:
     raw=(pathlib.Path('/proc')/pid/'stat').read_text();parts=raw[raw.rfind(')')+2:].split()
     processes.append(dict(key=pid+':'+parts[19],name=raw[raw.find('(')+1:raw.rfind(')')],ticks=int(parts[11])+int(parts[12]),rss_pages=int(parts[21])))
    except (FileNotFoundError,ProcessLookupError,PermissionError):pass
 result['processes']=processes;result['clock_ticks_per_second']=os.sysconf('SC_CLK_TCK')
 return result
usage_before=usage();wall=time.monotonic();active_before=None;active_after=None
with (output/'stderr.txt').open('w') as err:
 p=subprocess.Popen([str(host),str(bundle),'ap9-'+role,str(CASE['rate']),str(CASE['block']),str(CASE['seconds']),str(CASE['voices'])],env={**os.environ,'LD_PRELOAD':str(audit)},stdout=subprocess.PIPE,stderr=err,text=True)
 def timeout(*_):p.kill();raise TimeoutError('AP9 caller bound')
 signal.signal(signal.SIGALRM,timeout);signal.alarm(CASE['seconds']+60);lines=[]
 try:
  for line in p.stdout:
   lines.append(line)
   if '"event":"ap9_audio_begin"' in line:active_before=usage()
   if '"event":"ap9_audio_end"' in line:active_after=usage()
  code=p.wait(timeout=5)
 finally:signal.alarm(0)
record={'case':CASE,'returncode':code,'wall_seconds':time.monotonic()-wall,'unit_before':usage_before,'unit_after':usage(),'active_before':active_before,'active_after':active_after,'native_sha256':hashlib.sha256((bundle/('Contents/x86_64-linux/'+bundle.stem+'.so')).read_bytes()).hexdigest(),'stdout':''.join(lines),'stderr':(output/'stderr.txt').read_text()}
(output/'run.json').write_text(json.dumps(record))
new=set(results.glob('*'))-before
for path in new:
 if path.is_file() and path.suffix=='.jsonl':(output/path.name).write_bytes(path.read_bytes())
record['new_results']=[str(p) for p in new]
(output/'run.json').write_text(json.dumps(record))
print(json.dumps(record))

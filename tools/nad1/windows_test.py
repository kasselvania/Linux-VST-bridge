"""Source-owned fixed SCM fixture on an ephemeral Windows CI worker."""
import hashlib,os,pathlib,shutil,subprocess,sys,tempfile,time,struct

def run(build):
 build=pathlib.Path(build).resolve();root=pathlib.Path('C:/NAD1Fixture')
 if root.exists():raise ValueError('fixture_directory_already_exists')
 root.mkdir();shutil.copyfile(build/'nad1-service-fixture.exe',root/'Setup.exe')
 op=os.urandom(16).hex();token=os.urandom(32).hex();sha=hashlib.sha256((root/'Setup.exe').read_bytes()).hexdigest()
 anchor=None;generation=(0,0)
 def command(action):
  nonlocal anchor,generation
  request=root/'request';fields=['NAD1_SERVICE_V1',op,token,action,sha]
  if action=='stop':fields=['NAD1_STOP_REQUEST_V2',op,token,action,sha,*map(str,generation)]
  request.write_bytes(('\n'.join([*fields,''])).encode('utf-16le'))
  if action=='start':
   anchor=subprocess.Popen([str(build/'nad1-service-adapter.exe'),str(request)],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
   output=anchor.stdout.readline().decode()
   fields=output.split()
   if len(fields)==12 and fields[5]=='4':generation=(int(fields[6]),int(fields[7]))
   return output
  p=subprocess.run([str(build/'nad1-service-adapter.exe'),str(request)],capture_output=True,timeout=60)
  if p.returncode:raise ValueError(('adapter_failure',action,p.returncode))
  output=p.stdout.decode();rows=[line.split() for line in output.splitlines() if line.startswith('NAD1_SCM_V1 ')]
  if rows and rows[0][5]=='4':generation=(int(rows[0][6]),int(rows[0][7]))
  elif rows and rows[0][5]=='1':generation=(0,0)
  return output
 installed=False
 try:
  assert ' absent 1060 ' in command('query')
  assert subprocess.run([str(root/'Setup.exe')],timeout=10).returncode==1063
  assert 'NAD1_INSTALL_ROOT_V1 '+op in command('install');installed=True
  assert ' exact 0 1 ' in command('query')
  assert subprocess.run([str(root/'NTKDaemon.exe')],timeout=10).returncode==1063
  command('start');deadline=time.monotonic()+15
  while time.monotonic()<deadline:
   response=command('query')
   if ' exact 0 4 ' in response:break
   time.sleep(.1)
  assert ' exact 0 4 ' in response and sha in response and response.split()[-1]=='3'
  assert subprocess.run([str(build/'nad1-application-fixture.exe'),'--disable-gpu'],timeout=10).returncode==0
  assert (root/'application-started').exists()
  # Exact secondary-instance browser return through the same retained adapter.
  (root/'application-started').unlink();(root/'await-callback').touch()
  payload=build/'nad1-application-fixture.exe';callback_sha=hashlib.sha256(payload.read_bytes()).hexdigest()
  request=root/'application-request';request.write_bytes(('\n'.join(['NAUI2_AUTH_LAUNCH_V1',op,token,'2',callback_sha,str(payload.stat().st_size),str(payload),'software_rendering',''])).encode('utf-16le'))
  adapter=subprocess.Popen([str(build/'is2-launch.exe'),str(request)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
  try:
   deadline=time.monotonic()+15
   while not (root/'callback-ready').exists() and time.monotonic()<deadline:time.sleep(.05)
   assert (root/'callback-ready').exists()
   value=b'native-access:source-owned-fixture'
   out,err=adapter.communicate(value+b'\n',timeout=20)
   assert adapter.returncode==0,(adapter.returncode,out,err)
   assert (b'NA_AUTH_V1 '+op.encode()+b' 1 0') in out
   assert (root/'callback-received').exists()
   assert value not in out+err
  finally:
   if adapter.poll() is None:adapter.kill();adapter.wait(timeout=10)
  stopped=command('stop')
  assert 'NAD1_RETIRE_V1 '+op+' '+token+' 1 ' in stopped
  assert ' exact 0 1 0 0 none ' in stopped
  assert anchor.wait(timeout=10)==0
  assert ' exact 0 1 0 0 none ' in command('query')
  command('start');deadline=time.monotonic()+15
  while ' exact 0 4 ' not in command('query'):
   assert time.monotonic()<deadline;time.sleep(.05)
  assert 'NAD1_RETIRE_V1 '+op+' '+token+' 1 ' in command('stop')
  assert anchor.wait(timeout=10)==0
  # Slow clean shutdown must be observed rather than rejected at STOP_PENDING.
  (root/'delay-stop').touch();command('start')
  deadline=time.monotonic()+15
  while ' exact 0 4 ' not in command('query'):
   assert time.monotonic()<deadline;time.sleep(.05)
  stopped=command('stop');assert 'NAD1_RETIRE_V1 '+op+' '+token+' 1 ' in stopped
  assert anchor.wait(timeout=10)==0
  # An externally pending stop has already received its single control.
  command('start');deadline=time.monotonic()+15
  while ' exact 0 4 ' not in command('query'):
   assert time.monotonic()<deadline;time.sleep(.05)
  controls=(root/'events.private').read_text().count('stop_requested')
  subprocess.run(['sc.exe','stop','NAD1FixtureService'],check=True,capture_output=True,timeout=10)
  stopped=command('stop')
  observation=next(line.split()[3:] for line in stopped.splitlines() if line.startswith('NAD1_STOP_OBSERVATION_V2 '))
  assert observation[0]=='1' and observation[1]=='3' and observation[10]=='0',observation
  assert (root/'events.private').read_text().count('stop_requested')==controls+1
  assert anchor.wait(timeout=10)==0
 finally:
  if installed:subprocess.run([str(root/'Setup.exe'),'--remove'],check=True,timeout=15)
  shutil.rmtree(root)
if __name__=='__main__':run(sys.argv[1])

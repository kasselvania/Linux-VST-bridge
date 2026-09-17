"""Source-owned fixed SCM fixture on an ephemeral Windows CI worker."""
import hashlib,os,pathlib,shutil,subprocess,sys,tempfile,time,struct

def run(build):
 build=pathlib.Path(build).resolve();root=pathlib.Path('C:/NAD1Fixture')
 if root.exists():raise ValueError('fixture_directory_already_exists')
 root.mkdir();shutil.copyfile(build/'nad1-service-fixture.exe',root/'Setup.exe')
 op=os.urandom(16).hex();token=os.urandom(32).hex();sha=hashlib.sha256((root/'Setup.exe').read_bytes()).hexdigest()
 anchor=None
 def command(action):
  nonlocal anchor
  request=root/'request';request.write_bytes(('\n'.join(['NAD1_SERVICE_V1',op,token,action,sha,''])).encode('utf-16le'))
  if action=='start':
   anchor=subprocess.Popen([str(build/'nad1-service-adapter.exe'),str(request)],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
   return anchor.stdout.readline().decode()
  p=subprocess.run([str(build/'nad1-service-adapter.exe'),str(request)],capture_output=True,timeout=60)
  if p.returncode:raise ValueError(('adapter_failure',action,p.returncode))
  return p.stdout.decode()
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
  command('start')
  assert 'NAD1_RETIRE_V1 '+op+' '+token+' 1 ' in command('stop')
  assert anchor.wait(timeout=10)==0
 finally:
  if installed:subprocess.run([str(root/'Setup.exe'),'--remove'],check=True,timeout=15)
  shutil.rmtree(root)
if __name__=='__main__':run(sys.argv[1])

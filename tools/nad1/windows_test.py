"""Source-owned fixed SCM fixture on an ephemeral Windows CI worker."""
import hashlib,os,pathlib,shutil,subprocess,sys,tempfile,time

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
  assert ' exact 0 4 ' in response and sha in response
  command('stop')
  assert anchor.wait(timeout=10)==0
 finally:
  if installed:subprocess.run([str(root/'Setup.exe'),'--remove'],check=True,timeout=15)
  shutil.rmtree(root)
if __name__=='__main__':run(sys.argv[1])

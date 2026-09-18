"""Source-owned fixed SCM fixture on an ephemeral Windows CI worker."""
import hashlib,os,pathlib,shutil,subprocess,sys,tempfile,time,struct

def run(build):
 build=pathlib.Path(build).resolve();root=pathlib.Path('C:/NAD1Fixture')
 if root.exists():raise ValueError('fixture_directory_already_exists')
 root.mkdir();shutil.copyfile(build/'nad1-service-fixture.exe',root/'Setup.exe')
 op=os.urandom(16).hex();token=os.urandom(32).hex();sha=hashlib.sha256((root/'Setup.exe').read_bytes()).hexdigest()
 anchor=None;generation=(0,0)
 def command(action,expected=(0,)):
  nonlocal anchor,generation
  request=root/'request';fields=['NAD1_SERVICE_V1',op,token,action,sha]
  if action=='stop':fields=['NAD1_STOP_REQUEST_V2',op,token,action,sha,*map(str,generation)]
  request.write_bytes(('\n'.join([*fields,''])).encode('utf-16le'))
  if action=='start':
   anchor=subprocess.Popen([str(build/'nad1-service-adapter.exe'),str(request)],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
   output=anchor.stdout.readline().decode()
   fields=output.split()
   assert len(fields)==12 and fields[5]=='4',('start_ack_not_running',output)
   generation=(int(fields[6]),int(fields[7]))
   return output
  p=subprocess.run([str(build/'nad1-service-adapter.exe'),str(request)],capture_output=True,timeout=60)
  if p.returncode not in expected:raise ValueError(('adapter_failure',action,p.returncode,p.stdout,p.stderr))
  output=p.stdout.decode();rows=[line.split() for line in output.splitlines() if line.startswith('NAD1_SCM_V1 ')]
  if rows and rows[0][5]=='4':generation=(int(rows[0][6]),int(rows[0][7]))
  elif rows and rows[0][5]=='1':generation=(0,0)
  return output
 def characterization(output):
  rows=[line.split() for line in output.splitlines() if line.startswith('NAD2_STOP_CHARACTERIZATION_V1 ')]
  assert len(rows)==1 and len(rows[0])==45,rows
  values=list(map(int,rows[0][4:]))
  return {'classification':rows[0][3],'initial_state':values[1],'control_count':values[11],
   'control_submitted':values[12],'control_error':values[13],'control_state':values[16],
   'final_state':values[23],'process_wait':values[30],'listener_mask':values[32],
   'elapsed_ms':values[35],'progress_count':values[37],'transition_total':values[38],
   'transition_retained':values[39],'transition_dropped':values[40]}
 def wait_running():
  deadline=time.monotonic()+15
  while time.monotonic()<deadline:
   response=command('query')
   if ' exact 0 4 ' in response:return response
   time.sleep(.05)
  raise AssertionError(('service_not_running',response))
 def normal_after_adverse():
  nonlocal anchor
  command('start');wait_running();stopped=command('stop');observed=characterization(stopped)
  assert observed['classification']=='NAD2_STOP_CONFIRMED' and observed['control_count']==1,observed
  assert anchor.wait(timeout=15)==0
 installed=False
 try:
  # The production adapter also supplies the inert Windows root that keeps one
  # initialized Proton command session alive. It has no SCM authority and exits
  # only when its exact operation-private hold file is removed.
  runtime_request=root/'runtime-request'
  runtime_request.write_bytes(('\n'.join(['NAD1_RUNTIME_V1',op,token,''])).encode('utf-16le'))
  runtime_hold=pathlib.Path(str(runtime_request)+'.hold')
  runtime_hold.write_bytes(('\n'.join(['NAD1_RUNTIME_HOLD_V1',op,token,''])).encode('utf-16le'))
  runtime=subprocess.Popen([str(build/'nad1-service-adapter.exe'),str(runtime_request)],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
  runtime_frame=runtime.stdout.readline().decode().strip()
  assert runtime_frame==f'NAD1_RUNTIME_V1 {op} {token}' and runtime.poll() is None,runtime_frame
  runtime_hold.unlink();assert runtime.wait(timeout=10)==0
  runtime_hold.write_bytes(('\n'.join(['NAD1_RUNTIME_HOLD_V1',op,token,''])).encode('utf-16le'))
  replaced=subprocess.Popen([str(build/'nad1-service-adapter.exe'),str(runtime_request)],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
  replaced_frame=replaced.stdout.readline().decode().strip()
  assert replaced_frame==f'NAD1_RUNTIME_V1 {op} {token}' and replaced.poll() is None,replaced_frame
  replacement=pathlib.Path(str(runtime_hold)+'.replacement')
  replacement.write_bytes(runtime_hold.read_bytes());replacement.replace(runtime_hold)
  assert replaced.wait(timeout=10)==156
  runtime_hold.unlink()
  missing=subprocess.run([str(build/'nad1-service-adapter.exe'),str(runtime_request)],capture_output=True,timeout=10)
  assert missing.returncode==154 and b'NAD1_RUNTIME_V1 ' not in missing.stdout
  runtime_request.write_bytes(('\n'.join(['NAD1_RUNTIME_V1',op,'wrong',''])).encode('utf-16le'))
  refused=subprocess.run([str(build/'nad1-service-adapter.exe'),str(runtime_request)],input=b'',capture_output=True,timeout=10)
  assert refused.returncode==153 and b'NAD1_RUNTIME_V1 ' not in refused.stdout
  assert ' absent 1060 ' in command('query')
  assert subprocess.run([str(root/'Setup.exe')],timeout=10).returncode==1063
  assert 'NAD1_INSTALL_ROOT_V1 '+op in command('install');installed=True
  assert ' exact 0 1 ' in command('query')
  # The production request/parser path must preserve one transient exact 1060,
  # then observe the already-registered fixed service without mutation.
  (root/'query-absent-once').touch()
  assert ' absent 1060 ' in command('query')
  assert ' exact 0 1 ' in command('query')
  # A genuinely persistent absence remains absence across the single bounded
  # re-observation opportunity; removing the source-owned marker restores the
  # ordinary exact fixture state for all later lifecycle cases.
  (root/'query-absent-always').touch()
  assert ' absent 1060 ' in command('query')
  assert ' absent 1060 ' in command('query')
  (root/'query-absent-always').unlink()
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
  observed=characterization(stopped)
  assert observed['classification']=='NAD2_STOP_CONFIRMED' and observed['initial_state']==4 and observed['final_state']==1,observed
  assert observed['control_count']==1 and observed['control_submitted']==1 and observed['control_error']==0,observed
  assert anchor.wait(timeout=10)==0
  assert ' exact 0 1 0 0 none ' in command('query')
  command('start');deadline=time.monotonic()+15
  while ' exact 0 4 ' not in command('query'):
   assert time.monotonic()<deadline;time.sleep(.05)
  stopped=command('stop');assert 'NAD1_RETIRE_V1 '+op+' '+token+' 1 ' in stopped
  assert characterization(stopped)['classification']=='NAD2_STOP_CONFIRMED'
  assert anchor.wait(timeout=10)==0
  # Slow clean shutdown must be observed rather than rejected at STOP_PENDING.
  (root/'delay-stop').touch();command('start')
  deadline=time.monotonic()+15
  while ' exact 0 4 ' not in command('query'):
   assert time.monotonic()<deadline;time.sleep(.05)
  stopped=command('stop');assert 'NAD1_RETIRE_V1 '+op+' '+token+' 1 ' in stopped
  delayed_observation=characterization(stopped)
  assert delayed_observation['classification']=='NAD2_STOP_CONFIRMED',delayed_observation
  assert delayed_observation['transition_retained']==12 and delayed_observation['transition_dropped']>0,delayed_observation
  assert delayed_observation['transition_total']==delayed_observation['transition_retained']+delayed_observation['transition_dropped']
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
  pending_observation=characterization(stopped)
  assert pending_observation['classification']=='NAD2_STOP_CONFIRMED' and pending_observation['control_count']==0,pending_observation
  assert (root/'events.private').read_text().count('stop_requested')==controls+1
  assert anchor.wait(timeout=10)==0
  # Completed external stop: the anchor has released its process handle before
  # our stop helper starts. Keep the earlier RUNNING generation, not a new query.
  (root/'delay-stop').unlink();command('start');deadline=time.monotonic()+15
  while ' exact 0 4 ' not in command('query'):
   assert time.monotonic()<deadline;time.sleep(.05)
  admitted=generation;controls=(root/'events.private').read_text().count('stop_requested')
  subprocess.run(['sc.exe','stop','NAD1FixtureService'],check=True,capture_output=True,timeout=10)
  assert anchor.wait(timeout=15)==0
  receipt=root/f'{op}-{token}-{admitted[0]}-{admitted[1]}-anchor-exit.private'
  retained=receipt.read_bytes();assert retained==f'NAD1_ANCHOR_EXIT_V1 {op} {token} {admitted[0]} {admitted[1]} 0 0\n'.encode()
  # Wrong generation/nonce, truncation, extension and aliasing cannot substitute
  # for the original handle observation, even if an OS process object lingers.
  request=root/'request'
  fields=['NAD1_STOP_REQUEST_V2',op,token,'stop',sha,*map(str,admitted),'']
  request.write_bytes(('\n'.join(fields)).encode('utf-16le'))
  for bad in (retained.replace(token.encode(),b'0'*64),retained.replace(op.encode(),b'0'*32),retained.replace(str(admitted[0]).encode(),b'1'),retained.replace(str(admitted[1]).encode(),b'1'),retained[:-1],retained+b'x'):
   receipt.write_bytes(bad)
   p=subprocess.run([str(build/'nad1-service-adapter.exe'),str(request)],capture_output=True,timeout=20)
   assert p.returncode==149,(p.returncode,p.stdout)
   assert b'NAD1_EXIT_WITNESS_V1 ' not in p.stdout
  receipt.write_bytes(retained)
  alias=root/'receipt-alias';os.link(receipt,alias)
  p=subprocess.run([str(build/'nad1-service-adapter.exe'),str(request)],capture_output=True,timeout=20)
  assert p.returncode==149;alias.unlink()
  # A missing receipt and invalid process lookup cannot establish exit.
  missing=['NAD1_STOP_REQUEST_V2',op,token,'stop',sha,'4294967295',str(admitted[1]),'']
  request.write_bytes(('\n'.join(missing)).encode('utf-16le'))
  p=subprocess.run([str(build/'nad1-service-adapter.exe'),str(request)],capture_output=True,timeout=20)
  assert p.returncode==149 and b'NAD1_EXIT_WITNESS_V1 ' not in p.stdout and b'NAD1_STOP_BEGIN_V1 ' not in p.stdout
  generation=admitted
  stopped=command('stop')
  assert f'NAD1_EXIT_WITNESS_V1 {op} {token} {admitted[0]} {admitted[1]}' in stopped
  observation=next(line.split()[3:] for line in stopped.splitlines() if line.startswith('NAD1_STOP_OBSERVATION_V2 '))
  assert observation[0]=='1' and observation[1]=='1' and observation[10]=='0',observation
  assert (root/'events.private').read_text().count('stop_requested')==controls+1
  assert receipt.read_bytes()==retained
  # Adverse service responses cross the production adapter and are followed by
  # a fresh ordinary operation so fixture cleanup cannot hide poisoned state.
  def release_adverse(*names):
   (root/'release-stop').touch()
   time.sleep(.25)  # Let the held service observe release even if its anchor already timed out.
   assert anchor.wait(timeout=20) in (0,150)
   for name in (*names,'release-stop'):(root/name).unlink(missing_ok=True)
   assert ' exact 0 1 ' in command('query')

  (root/'no-transition').touch();command('start');wait_running()
  unchanged=characterization(command('stop',expected=(149,)))
  assert unchanged['classification']=='NAD2_STOP_SUBMITTED_NO_TRANSITION',unchanged
  assert unchanged['control_count']==1 and unchanged['control_submitted']==1 and unchanged['progress_count']==0,unchanged
  assert unchanged['process_wait']==258 and unchanged['listener_mask']==3 and unchanged['elapsed_ms']>=12000,unchanged
  release_adverse('no-transition');normal_after_adverse()

  (root/'control-failure').touch();command('start');wait_running()
  refused=characterization(command('stop',expected=(149,)))
  assert refused['classification']=='NAD2_STOP_NOT_SUBMITTED' and refused['control_count']==1,refused
  assert refused['control_submitted']==0 and refused['control_error']!=0,refused
  release_adverse('control-failure');normal_after_adverse()

  (root/'query-failure').touch();command('start');wait_running()
  unavailable=characterization(command('stop',expected=(149,)))
  assert unavailable['classification']=='NAD2_STOP_OBSERVATION_UNAVAILABLE',unavailable
  (root/'query-failure').unlink();assert anchor.wait(timeout=15)==0
  normal_after_adverse()

  for hold,expected_mask in [('stopped-process-hold',0),('stopped-listener-hold',3)]:
   (root/'release-stop').unlink(missing_ok=True)
   (root/hold).touch();command('start');wait_running()
   residue=characterization(command('stop',expected=(149,)))
   assert residue['classification']=='NAD2_STOPPED_PROCESS_OR_LISTENER_REMAINS',residue
   assert residue['final_state']==1 and residue['listener_mask']==expected_mask,residue
   release_adverse(hold);normal_after_adverse()

  (root/'pending-hold').touch();(root/'pending-progress').touch();command('start');wait_running()
  controls=(root/'events.private').read_text().count('stop_requested')
  subprocess.run(['sc.exe','stop','NAD1FixtureService'],check=True,capture_output=True,timeout=10)
  progressing=characterization(command('stop',expected=(149,)))
  assert progressing['classification']=='NAD2_STOP_SUBMITTED_PROGRESSING',progressing
  assert progressing['initial_state']==3 and progressing['control_count']==0 and progressing['progress_count']>0,progressing
  assert (root/'events.private').read_text().count('stop_requested')==controls+1
  release_adverse('pending-hold','pending-progress');normal_after_adverse()
 finally:
  if root.exists():
   (root/'release-stop').touch()
   for name in ('no-transition','refuse-stop','control-failure','query-failure','stopped-process-hold','stopped-listener-hold','pending-hold','pending-progress','delay-stop','self-stop'):
    (root/name).unlink(missing_ok=True)
  if installed:subprocess.run([str(root/'Setup.exe'),'--remove'],check=True,timeout=15)
  shutil.rmtree(root)
if __name__=='__main__':run(sys.argv[1])

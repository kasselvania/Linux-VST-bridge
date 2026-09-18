"""Bounded, non-secret failed-stop observations survive the login privacy cutoff."""
import json,pathlib,subprocess,sys,tempfile,unittest
from unittest.mock import patch
import session as s
class StopReporting(unittest.TestCase):
 def values(self,**changes):
  v=dict(zip(s.NAD1_STOP_FIELDS,[0,4,0,3,0,5,2000,258,0,3,1,0,2,300,12000,240,5,0,0,0]));v.update(changes);return v
 def frame(self,op='a'*32,token='b'*64,**changes):
  v=self.values(**changes)
  return ('NAD1_STOP_OBSERVATION_V2 '+op+' '+token+' '+' '.join(str(v[k]) for k in s.NAD1_STOP_FIELDS)+'\n').encode()
 def characterized(self,classification='NAD2_STOP_SUBMITTED_PROGRESSING',op='a'*32,token='b'*64,**changes):
  legacy_changes={key:value for key,value in changes.items() if key in s.NAD1_STOP_FIELDS};v=self.values(**legacy_changes)
  n={'initial_service_type':16,'initial_state':v['initial_state'],'initial_controls_accepted':1,
   'initial_checkpoint':0,'initial_wait_hint_ms':0,'initial_service_exit':0,'initial_service_specific_exit':0,
   'initial_query_error':v['initial_query_error'],'initial_process_wait':258,'initial_process_wait_error':0,'initial_listener_mask':3,
   'control_count':v['control_sent'],'control_submitted':v['control_sent'],'control_error':v['control_error'],
   'control_status_available':v['control_sent'],'control_service_type':16 if v['control_sent'] else 0,
   'control_state':3 if v['control_sent'] else 0,'control_controls_accepted':0,'control_checkpoint':1 if v['control_sent'] else 0,
   'control_wait_hint_ms':1000 if v['control_sent'] else 0,'control_service_exit':0,'control_service_specific_exit':0,
   'final_service_type':16,'final_state':v['final_state'],'final_controls_accepted':0,'final_checkpoint':v['checkpoint'],
   'final_wait_hint_ms':v['wait_hint_ms'],'final_service_exit':v['service_exit'],'final_service_specific_exit':v['service_specific_exit'],
   'final_query_error':v['final_query_error'],'process_wait':v['process_wait'],'process_wait_error':v['process_wait_error'],
   'listener_mask':v['endpoint_mask'],'control_started_ms':v['control_started_ms'],'control_elapsed_ms':v['control_elapsed_ms'],
   'elapsed_ms':v['elapsed_ms'],'query_count':v['query_count'],'progress_count':v['progress_count'],
   'transition_count_total':2,'transition_count_retained':2,'transition_count_dropped':0}
  n.update({key:value for key,value in changes.items() if key in s.NAD2_STOP_FIELDS})
  if not n['control_submitted']:
   n.update(control_status_available=0,control_service_type=0,control_state=0,control_controls_accepted=0,
    control_checkpoint=0,control_wait_hint_ms=0,control_service_exit=0,control_service_specific_exit=0)
  first_wait={0:1,258:2,0xffffffff:3}.get(n['initial_process_wait'],4)
  last_wait={0:1,258:2,0xffffffff:3}.get(n['process_wait'],4)
  transitions=[[1,1,0,n['initial_service_type'],n['initial_state'],n['initial_controls_accepted'],n['initial_checkpoint'],
   n['initial_wait_hint_ms'],n['initial_service_exit'],n['initial_service_specific_exit'],n['initial_query_error'],first_wait,
   n['initial_process_wait_error'],n['initial_listener_mask']]]
  if n['transition_count_retained']>1:
   transitions.append([2,1,n['elapsed_ms'],n['final_service_type'],n['final_state'],n['final_controls_accepted'],n['final_checkpoint'],
    n['final_wait_hint_ms'],n['final_service_exit'],n['final_service_specific_exit'],n['final_query_error'],last_wait,
    n['process_wait_error'],n['listener_mask']])
  old=self.frame(op,token,**legacy_changes)
  summary='NAD2_STOP_CHARACTERIZATION_V1 '+op+' '+token+' '+classification+' '+' '.join(str(n[key]) for key in s.NAD2_STOP_FIELDS)+'\n'
  ledger=''.join('NAD2_STOP_TRANSITION_V1 '+op+' '+token+' '+' '.join(map(str,row))+'\n' for row in transitions)
  return old+summary.encode()+ledger.encode()
 def test_closed_failure_and_success_schema(self):
  v=s.nad1_stop_observation(self.frame(),'a'*32,'b'*64)
  self.assertEqual(v['final_state'],3);self.assertEqual(v['checkpoint'],5);self.assertEqual(v['endpoint_mask'],3)
  self.assertEqual(v['control_elapsed_ms'],300);self.assertNotIn('windows_pid',v)
  good=self.frame(confirmed=1,final_state=1,process_wait=0,endpoint_mask=0)
  self.assertEqual(s.nad1_stop_observation(good,'a'*32,'b'*64)['confirmed'],1)
  for field,value in [('confirmed',2),('control_sent',2),('final_state',8),('endpoint_mask',5),('process_wait',1),('elapsed_ms',1),('query_count',-1)]:
   with self.subTest(field=field),self.assertRaises(ValueError):s.nad1_stop_observation(self.frame(**{field:value}),'a'*32,'b'*64)
  for field in ('final_query_error','identity_error','process_wait','endpoint_mask'):
   with self.subTest(field=field),self.assertRaises(ValueError):s.nad1_stop_observation(self.frame(confirmed=1,final_state=1,**{field:1}),'a'*32,'b'*64)
  for raw in [good+good,good.replace(b'a'*32,b'c'*32),good.replace(b'12000',b'4294967296'),good.replace(b'12000',b'false')]:
   with self.assertRaises(ValueError):s.nad1_stop_observation(raw,'a'*32,'b'*64)
 def test_every_nad2_classification_crosses_the_production_parser(self):
  cases={
   'NAD2_STOP_CONFIRMED':dict(confirmed=1,final_state=1,checkpoint=0,wait_hint_ms=0,process_wait=0,endpoint_mask=0,
    progress_count=2,initial_process_wait=258,initial_listener_mask=3),
   'NAD2_STOP_SUBMITTED_PROGRESSING':{},
   'NAD2_STOP_SUBMITTED_NO_TRANSITION':dict(final_state=4,checkpoint=0,wait_hint_ms=0,progress_count=0,
    transition_count_total=1,transition_count_retained=1),
   'NAD2_STOP_NOT_SUBMITTED':dict(final_state=4,checkpoint=0,wait_hint_ms=0,progress_count=0,control_error=5,
    control_submitted=0,transition_count_total=1,transition_count_retained=1),
   'NAD2_STOPPED_PROCESS_OR_LISTENER_REMAINS':dict(final_state=1,checkpoint=0,wait_hint_ms=0,process_wait=258,
    endpoint_mask=0,progress_count=2),
   'NAD2_STOP_OBSERVATION_UNAVAILABLE':dict(final_query_error=5,final_state=0,checkpoint=0,wait_hint_ms=0,
    process_wait=0xffffffff,process_wait_error=0,endpoint_mask=4,progress_count=0)}
  for classification,changes in cases.items():
   with self.subTest(classification=classification):
    value=s.nad1_stop_observation(self.characterized(classification,**changes),'a'*32,'b'*64)
    self.assertEqual(value['schema'],3);self.assertEqual(value['classification'],classification)
    self.assertNotIn('windows_pid',value);self.assertNotIn('token',value)
    if value['control_submitted']:
     self.assertTrue(value['control_return']['submitted']);self.assertIn('controls_accepted',value['control_return'])
  no_transition=s.nad1_stop_observation(self.characterized('NAD2_STOP_SUBMITTED_NO_TRANSITION',final_state=4,
   checkpoint=0,wait_hint_ms=0,progress_count=0,transition_count_total=1,transition_count_retained=1),'a'*32,'b'*64)
  self.assertEqual(no_transition['transition_count_dropped'],0);self.assertEqual(no_transition['process_wait_class'],'timeout')
  for raw in [self.characterized('NAD2_STOP_CONFIRMED'),
   self.characterized(transition_count_total=3,transition_count_retained=2,transition_count_dropped=0),
   self.characterized(control_status_available=0)]:
   with self.assertRaises(ValueError):s.nad1_stop_observation(raw,'a'*32,'b'*64)
 def test_failed_stop_keeps_structured_facts_with_raw_capture_disabled(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=pathlib.Path(tmp);(root/'home').mkdir()
   spec={'operation':'a'*32,'report':str(root/'result.json'),'application':{'environment':{'root':str(root),'runner':{}}},'installer_launch':{'path':'unused'}}
   class Ledger:
    def launcher(self,child,_):self.child=child
    def harvest(self):self.child.poll()
   owner=s.Nad1Owner(spec,Ledger(),None,lambda:False);owner.token='b'*64;owner.diagnostic_privacy=True
   owner.service_generation=(123,456)
   frame=self.characterized()
   class Runtime:
    ready=True;closed=False;SERVICE_SHA=s.Nad1Runtime.SERVICE_SHA
    def argv(self,*_):return [sys.executable,'-c',f'import os;os.write(2,b"LOGIN_SECRET");os.write(1,{frame!r});raise SystemExit(149)']
    def drain(self):pass
   owner.runtime=Runtime()
   with patch.object(s,'environment',return_value={}):
    with self.assertRaisesRegex(ValueError,'dependency_command_nonzero'):owner.command('stop')
   self.assertEqual(owner.stop_observation['checkpoint'],5)
   self.assertEqual(owner.stages[0]['stop_observation'],owner.stop_observation)
   self.assertEqual((root/('a'*32+'-dependency-0.private')).read_bytes().decode('utf-16le').splitlines()[-2:],['123','456'])
   self.assertEqual((root/('a'*32+'-dependency-0.log')).read_bytes(),b'')
   self.assertEqual(json.loads((root/('a'*32+'-retirement.private.json')).read_bytes()),{'raw_diagnostics_suppressed':True})
   self.assertNotIn('LOGIN_SECRET',json.dumps(owner.value()))
 def test_anchor_exit_handoff_remains_bound_and_private(self):
  for mutation in ('none','inactive','generation','nonce','duplicate'):
   with self.subTest(mutation=mutation),tempfile.TemporaryDirectory() as tmp:
    root=pathlib.Path(tmp);(root/'home').mkdir();op='a'*32;token='b'*64
    spec={'operation':op,'report':str(root/'result.json'),'application':{'environment':{'root':str(root),'runner':{}}},'installer_launch':{'path':'unused'}}
    class Ledger:
     def launcher(self,child,_):self.child=child
     def harvest(self):self.child.poll()
    owner=s.Nad1Owner(spec,Ledger(),None,lambda:False);owner.token=token;owner.diagnostic_privacy=True;owner.service_generation=(123,456)
    witness=f'NAD1_EXIT_WITNESS_V1 {op} {token} 123 456\n'
    if mutation=='nonce':witness=witness.replace(token,'c'*64)
    if mutation=='duplicate':witness*=2
    if mutation=='inactive':owner.service_generation=None;witness=''
    generation='0 0' if mutation=='inactive' else '124 456' if mutation=='generation' else '123 456'
    frames=self.frame(confirmed=1,initial_state=1,final_state=1,process_wait=0,endpoint_mask=0,control_sent=0,control_started_ms=0,control_elapsed_ms=0)+witness.encode()+f'NAD1_RETIRE_V1 {op} {token} 1 {generation} 0\nNAD1_SCM_V1 {op} {token} exact 0 1 0 0 none 0 0 0\n'.encode()
    class Runtime:
     ready=True;closed=False;SERVICE_SHA=s.Nad1Runtime.SERVICE_SHA
     def argv(self,*_):return [sys.executable,'-c',f'import os;os.write(1,{frames!r})']
     def drain(self):pass
    owner.runtime=Runtime()
    with patch.object(s,'environment',return_value={}):
     if mutation in ('none','inactive'):
      self.assertTrue(owner.command('stop')['retirement_generation_confirmed'])
      self.assertEqual(owner.stages[0]['retirement_generation_authority'],'exact_already_inactive' if mutation=='inactive' else 'retained_anchor_exit')
     else:
      with self.assertRaises(ValueError):owner.command('stop')
    self.assertNotIn('NAD1_EXIT_WITNESS',json.dumps(owner.value()))
    self.assertEqual((root/(op+'-dependency-0.log')).read_bytes(),b'')

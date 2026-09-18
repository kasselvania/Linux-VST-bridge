"""Bounded, non-secret failed-stop observations survive the login privacy cutoff."""
import json,pathlib,subprocess,sys,tempfile,unittest
from unittest.mock import patch
import session as s
class StopReporting(unittest.TestCase):
 def frame(self,op='a'*32,token='b'*64,**changes):
  v=dict(zip(s.NAD1_STOP_FIELDS,[0,4,0,3,0,5,2000,258,0,3,1,0,2,300,12000,240,5,0,0,0]));v.update(changes)
  return ('NAD1_STOP_OBSERVATION_V2 '+op+' '+token+' '+' '.join(str(v[k]) for k in s.NAD1_STOP_FIELDS)+'\n').encode()
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
 def test_failed_stop_keeps_structured_facts_with_raw_capture_disabled(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=pathlib.Path(tmp);(root/'home').mkdir()
   spec={'operation':'a'*32,'report':str(root/'result.json'),'application':{'environment':{'root':str(root),'runner':{}}},'installer_launch':{'path':'unused'}}
   class Ledger:
    def launcher(self,child,_):self.child=child
    def harvest(self):self.child.poll()
   owner=s.Nad1Owner(spec,Ledger(),None,lambda:False);owner.token='b'*64;owner.diagnostic_privacy=True
   owner.service_generation=(123,456)
   frame=self.frame()
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

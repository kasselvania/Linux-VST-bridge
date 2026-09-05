"""Focused rejection tests; actual SDK-host tests live beside the native code."""
import pathlib,sys
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parent))
import copy,unittest
from unittest.mock import patch
import ap3_contract as c
class AP3ContractTests(unittest.TestCase):
 def records(self):
  out=[]
  for n in (128,256):
   out += [dict(event='ap3_host_ready',seed=123,seed_after_activation=True),dict(event='ap3_host_compared',frames_per_callback=n,active_frames=1440000,samples=2882048,latency_samples=1024,max_error=0.,callback_overruns=0,callback_effects=0,fault_observed=False,callback_median_ns=1000,callback_p99_ns=2000,callback_max_ns=4000)]
  return out+[dict(event='ap3_proxy_stats',fault=0,processed=16887,position=1441024,epoch=2,request_high=8,result_high=8),dict(event='ap3_host_closed',terminate_result=0,module_unloaded=True,references_released=True)]
 def test_failed_or_incomplete_measurements_never_pass(self):
  with patch.object(c,'reconstruct',return_value={}):
   self.assertEqual(c.compare(self.records())['samples_compared'],5764096)
   for key,value in [('max_error',.001),('callback_overruns',1),('callback_effects',1),('samples',2882047),('latency_samples',0),('fault_observed',True),('callback_max_ns',100000000)]:
    with self.subTest(key=key):
     records=self.records();records[1][key]=value
     with self.assertRaises(ValueError):c.compare(records)
   for key,value in [('fault',1),('epoch',1),('processed',16886),('request_high',2049)]:
    records=self.records();records[-2][key]=value
    with self.assertRaises(ValueError):c.compare(records)
 def test_wrong_actual_sample_digest_fails(self):
  records=self.records();records[1]['output_fnv1a64']=1
  with patch.object(c,'reconstruct',return_value={'output_fnv1a64':2}):
   with self.assertRaises(ValueError):c.compare(records)
 def test_failure_detail_redacts_private_data_without_erasing_operation(self):
  from pc0_diagnostic_runtime import checkpoint_projection
  v=checkpoint_projection({'detail':'control disconnected/IO at /home/private-user/secret token=abc123 100.99.11.22','environment':{'password':'not retained'}})
  self.assertIn('control disconnected',v['detail']);self.assertNotIn('private-user',str(v));self.assertNotIn('abc123',str(v));self.assertNotIn('100.99',str(v));self.assertNotIn('environment',v)
 def test_public_observation_excludes_private_supervision_metadata(self):
  v={k:None for k in ('raw_exit','classification','cleanup','run_id','inherited_shutdown','stdout_sha256','stderr_sha256')}
  v.update(records=[],topology={'pid':123},runner_identity={'private_path':'secret'},protected_snapshot={'private':'secret'},caller=dict(raw_exit=0,cleanup=c.CLEAN,records=[],stderr_detail='private launcher data'))
  projected=c.public_observation(v,[],[dict(sequence=1,state='scanner_completed')])
  self.assertFalse({'topology','runner_identity','protected_snapshot'}&projected.keys())
  self.assertEqual(set(projected['caller']),{'raw_exit','cleanup','records'})
  self.assertNotIn('private',str(projected))
 def test_bitwig_requires_native_control_and_direct_gui_observations(self):
  q=dict(event='ap3_proxy_stats',fault=0,processed=5000,epoch=1,request_high=8,result_high=8)
  l=dict(event='ap3_proxy_lifecycle',blocks=5000,callback_rejections=0,frames=1280000,clean=True,requested_rate=48000,requested_maximum=256,requested_mode=0,gain_min=0,gain_max=1,zero_gain_blocks=100)
  u=dict(event='ap3_bitwig_ui',case='bitwig_first',**{k:True for k in ('scan_load','playback','gain_changed','muted','stopped','removed','responsive','moonlight_control')})
  records=[q,l,u];self.assertFalse(c.compare_gui(records,'bitwig_first')['numerical_oracle'])
  for index,key,value in [(0,'fault',1),(0,'processed',4999),(1,'clean',False),(1,'callback_rejections',1),(1,'frames',10),(1,'requested_maximum',512),(1,'gain_min',.5),(1,'zero_gain_blocks',0),(2,'playback',False),(2,'case','bitwig_reopen')]:
   bad=copy.deepcopy(records);bad[index][key]=value
   with self.subTest(key=key),self.assertRaises(ValueError):c.compare_gui(bad,'bitwig_first')
 def test_zero_sign_is_numerically_canonical(self):
  self.assertEqual(c.fold(123,0.),c.fold(123,-0.))
if __name__=='__main__':unittest.main()

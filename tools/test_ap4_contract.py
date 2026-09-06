"""Reference state capture and unchanged-project restore through AP4 admission."""
import pathlib,sys,copy,hashlib,struct,unittest
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parent))
from unittest.mock import patch
import ap4_contract as c

def state(operation,gain,reduction=0):
 return dict(event='ap4_native_state',operation=operation,gain=gain,payload_bytes=12,sha256=hashlib.sha256(struct.pack('<ffi',gain,reduction,0)).hexdigest())

def records(label):
 restored=.25 if label=='bitwig_gain' else 0.
 native=[state('get',1.)]
 if label!='bitwig_first':native.append(state('set',restored))
 if label!='bitwig_mute':native.append(state('get',.25 if label=='bitwig_first' else 0.))
 before={'bitwig_first':'a','bitwig_gain':'b','bitwig_mute':'c'}[label]*64
 after={'bitwig_first':'b','bitwig_gain':'c','bitwig_mute':'c'}[label]*64
 return native+[
  dict(event='ap4_bitwig_ui',case=label,**{k:True for k in ('playback','saved','quit','responsive','moonlight_control','restored_control','restored_playback','observed_before_edit','later_edit')}),
  dict(event='ap3_proxy_lifecycle',blocks=500,frames=128000,clean=True,callback_rejections=0,requested_rate=48000,requested_maximum=256),
  dict(event='ap3_proxy_stats',fault=0,processed=500,epoch=1),
  dict(event='ap4_sample_comparison',samples=256000,maximum_error=0.,restores=int(label!='bitwig_first'),restored_gain=restored,before_edit_samples=96000,restored_samples=256000,nonzero_samples=48000),
  dict(event='ap4_project',project_before_sha256=before,expected_before_sha256=before,project_sha256=after)]

class AP4ContractTests(unittest.TestCase):
 def test_final_reopen_uses_actual_restore_without_resaving_project(self):
  r=records('bitwig_mute');next(v for v in r if v['event']=='ap4_bitwig_ui')['saved']=False
  result=c.compare_gui(r,'bitwig_mute')
  self.assertFalse(any(v['operation']=='get' and v['gain']==0 for v in result['states']))
  self.assertTrue(any(v['operation']=='set' and v['gain']==0 for v in result['states']))
 def test_capture_still_requires_fresh_actual_snapshot(self):
  for label in ('bitwig_first','bitwig_gain'):
   r=[v for v in records(label) if not(v['event']=='ap4_native_state' and v['operation']=='get' and v['gain']!=1)]
   with self.subTest(label=label),self.assertRaisesRegex(ValueError,'saved processor'):c.compare_gui(r,label)
 def test_missing_restore_changed_project_and_wrong_audio_remain_rejected(self):
  for kind in ('restore','project','audio','before_edit','cleanup'):
   r=records('bitwig_mute')
   if kind=='restore':r=[v for v in r if not(v['event']=='ap4_native_state' and v['operation']=='set')]
   else:
    event,key,value={'project':('ap4_project','project_sha256','d'*64),'audio':('ap4_sample_comparison','maximum_error',.01),'before_edit':('ap4_sample_comparison','before_edit_samples',0),'cleanup':('ap3_proxy_lifecycle','clean',False)}[kind]
    next(v for v in r if v['event']==event)[key]=value
   with self.subTest(kind=kind),self.assertRaises(ValueError):c.compare_gui(r,'bitwig_mute')
 def test_native_state_digest_requires_actual_windows_readback(self):
  native=records('bitwig_mute')
  windows=[dict(state='ap4_state_started',operation='set',owner_thread=True),dict(state='ap4_state_result',operation='set',result=0)]
  windows += [dict(state='ap4_state_readback',payload_bytes=12,payload_hex=struct.pack('<ffi',gain,0.,0).hex()) for gain in (1.,0.)]
  observed=dict(caller=dict(raw_exit=0,cleanup=c.CLEAN,records=native),records=windows)
  with patch.object(c.previous,'validate_windows_session',side_effect=lambda o,comparison,label,ap4:dict(comparison=comparison)):
   c.normalize_session(observed,'bitwig_mute')
   native[1]['sha256']=state('set',0.,.125)['sha256']
   with self.assertRaisesRegex(ValueError,'not corroborated'):c.normalize_session(observed,'bitwig_mute')

 def test_native_decimal_readback_matches_exact_float32_bits(self):
  native=records('bitwig_mute')
  payload=struct.pack('<ffi',.505,0.,0)
  actual=struct.unpack('<f',payload[:4])[0]
  logged=float(format(actual,'.9g'))
  self.assertNotEqual(logged,actual)
  snapshot=dict(event='ap4_native_state',operation='get',gain=logged,payload_bytes=12,sha256=hashlib.sha256(payload).hexdigest())
  native.append(snapshot)
  windows=[dict(state='ap4_state_started',operation='set',owner_thread=True),dict(state='ap4_state_result',operation='set',result=0)]
  windows += [dict(state='ap4_state_readback',payload_bytes=12,payload_hex=struct.pack('<ffi',gain,0.,0).hex()) for gain in (1.,0.,.505)]
  observed=dict(caller=dict(raw_exit=0,cleanup=c.CLEAN,records=native),records=windows)
  with patch.object(c.previous,'validate_windows_session',side_effect=lambda o,comparison,label,ap4:dict(comparison=comparison)):
   result=c.normalize_session(observed,'bitwig_mute')
   self.assertEqual(result['comparison']['states'][-1],snapshot)
   # A numerically close but different float32 must still fail: no tolerance.
   adjacent=struct.unpack('<f',struct.pack('<I',struct.unpack('<I',payload[:4])[0]+1))[0]
   for invalid in (adjacent,.5,float('nan'),float('inf'),-1.,2.,True):
    snapshot['gain']=invalid
    with self.subTest(invalid=invalid),self.assertRaises(ValueError):c.normalize_session(observed,'bitwig_mute')
   snapshot['gain']=logged
   snapshot['sha256']=hashlib.sha256(struct.pack('<ffi',.505,.125,0)).hexdigest()
   with self.assertRaisesRegex(ValueError,'not corroborated'):c.normalize_session(observed,'bitwig_mute')

 def test_saved_full_payload_must_match_fresh_restore(self):
  sessions={label:c.compare_gui(records(label),label) for label in c.GUI};c.join_gui_states(sessions)
  for field in ('gain','hidden_reduction'):
   bad=copy.deepcopy(sessions)
   restored=next(r for r in bad['bitwig_mute']['states'] if r['operation']=='set')
   restored['sha256']=state('set',.25 if field=='gain' else 0.,.125 if field=='hidden_reduction' else 0.)['sha256']
   with self.subTest(field=field),self.assertRaisesRegex(ValueError,'state bytes differ'):c.join_gui_states(bad)
if __name__=='__main__':unittest.main()

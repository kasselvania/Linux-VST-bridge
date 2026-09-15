import unittest
from pathlib import Path
from analysis import delta32,unfold32,queue_words,classify,INSUFFICIENT

class UIR2Tests(unittest.TestCase):
 def facts(self,**kw):
  return dict(dict(contacts=1,coverage_complete=True,clock_validated=True,procedure_after_removal=True,
      removal_upper_ms=30,procedure_gap_upper_ms=2),**kw)
 def test_wrap(self):
  self.assertEqual(delta32(3,0xfffffffe),5)
  self.assertEqual(delta32(0xfffffffe,3),-5)
  self.assertEqual(unfold32(3,0xfffffffe),0x100000003)
  with self.assertRaises(ValueError):delta32(0x80000000,0)
  for x in (True,-1,2**32,1.1):
   with self.assertRaises(ValueError):delta32(x,0)
 def test_queue_is_hint_and_words(self):
  q=queue_words((0x1000<<16)|8)
  self.assertEqual(q,dict(changed=8,current=0x1000,admission_proved=False))
  self.assertEqual(classify(self.facts(removal_upper_ms=2000,queue_hint=0x1000)),INSUFFICIENT)
 def test_retrieval_requires_exact_availability(self):
  f=self.facts(removal_upper_ms=1100,removal_lower_ms=1000,available_upper_ms=20,exact_release_available=True)
  self.assertEqual(classify(f),'UIR2_HOST_POINTER_RETRIEVAL_DELAY')
  del f['exact_release_available'];self.assertEqual(classify(f),INSUFFICIENT)
 def test_admission_requires_creation_not_inherited_timestamp(self):
  f=self.facts(removal_upper_ms=2000,creation_clock_validated=True,creation_lower_ms=1500,
     exact_release_absent_until_creation=True,pump_live=True)
  self.assertEqual(classify(f),'UIR2_WINE_XWAYLAND_ADMISSION_DELAY')
  f['creation_clock_validated']=False;self.assertEqual(classify(f),INSUFFICIENT)
 def test_prompt_minimal_selects_embedded(self):
  self.assertEqual(classify(self.facts()),'UIR2_EMBEDDED_PRODUCT_ROUTE_REQUIRED')
 def test_sent_hook_cannot_replace_procedure(self):
  self.assertEqual(classify(self.facts(procedure_after_removal=False,sent_hook=True)),INSUFFICIENT)
 def test_posted_heartbeat_not_pointer_availability(self):
  self.assertEqual(queue_words(0x00080008)['current']&0x1804,0)
  self.assertEqual(classify(self.facts(removal_upper_ms=15000,pump_live=True)),INSUFFICIENT)
 def test_multicontact_and_clock_ambiguity(self):
  for n in (0,2,3):self.assertEqual(classify(self.facts(contacts=n)),INSUFFICIENT)
  self.assertEqual(classify(self.facts(clock_validated=False)),INSUFFICIENT)
 def test_actual_pump_and_no_input_injection(self):
  src=(Path(__file__).parent/'windows.cpp').read_text()
  self.assertIn('#include "vendor_view.h"',src);self.assertIn('VendorView::pump()',src)
  self.assertIn('const auto ok=::PeekMessageW(msg,w,first,last,flags)',src)
  self.assertIn('const auto result=::DispatchMessageW(msg)',src)
  for call in ('InjectTouchInput(', 'SendInput(', 'SetWindowsHookEx', 'SetWindowSubclass','XTestFake'):
   self.assertNotIn(call,src)
  self.assertIn('if(n>=capacity)',src);self.assertIn('DestroyWindow(root)',src)
  self.assertIn('UnmapViewOfFile(h)',src)
 def test_missing_coverage(self):
  self.assertEqual(classify(self.facts(coverage_complete=False)),INSUFFICIENT)

if __name__=='__main__':unittest.main()

class SessionTests(unittest.TestCase):
 def raw(self):
  # Millisecond clock at 1000; a single independently scoped contact. Distinct
  # Linux and Windows identities are intentionally not equated.
  return dict(schema=1,action_begin_ns=999000000,raw=[
   dict(kind='raw_touch_begin',device=1,source=2,detail=44,observed_ns=1000000000),
   dict(kind='raw_touch_end',device=1,source=2,detail=44,observed_ns=1001000000)],
   x11=[dict(kind='core_up',server_ms=1000,observed_ns=1001000000)],
   windows=[dict(kind=2,value=1,message=0x247,hwnd=73,qpc=1010,end_qpc=1011,message_time=1000),
            dict(kind=4,message=0x247,hwnd=73,qpc=1012,dispatch=7),
            dict(kind=3,message=0x247,hwnd=73,qpc=1012,end_qpc=1013,dispatch=7)],
   windows_clocks=[dict(linux_before_ns=999000000,linux_after_ns=1001000000,windows_qpc=1000,tick=1000,frequency=1000)]*2,
   x_clocks=[dict(linux_before_ns=999000000,linux_after_ns=1001000000,server_ms=1000)]*2,
   status=dict(frequency=1000,calibration_ok=1),drops={},error=None,final_pointer=dict(mask=0))
 def test_full_summary_prompt(self):
  from session import summary
  s=summary(self.raw());self.assertEqual(s['disposition'],'UIR2_EMBEDDED_PRODUCT_ROUTE_REQUIRED')
  self.assertEqual(s['procedure_gaps_ms'],[1.0])
 def test_summary_missing_and_conflicting(self):
  from session import summary
  for mutate in (lambda r:r['raw'][1].update(source=3),lambda r:r.update(error='cleanup'),
                 lambda r:r['drops'].update(windows=1),lambda r:r['windows_clocks'].clear(),
                 lambda r:r['windows'][1].update(dispatch=0)):
   r=self.raw();mutate(r);self.assertEqual(summary(r)['disposition'],INSUFFICIENT)
 def test_mapping_commit_and_closed_mailbox(self):
  import tempfile,struct
  from session import Mapping,HEADER,SIZE,RECORD
  with tempfile.TemporaryDirectory() as d:
   p=Path(d)/'status'
   with p.open('wb') as f:f.truncate(SIZE)
   with p.open('r+b') as f:
    f.write(struct.pack('<3Q',0x32524955,1,SIZE))
   m=Mapping(p)
   try:
    with self.assertRaises(ValueError):m.write('pid',1)
    m.write('stop',1);self.assertEqual(m.read('stop'),1)
    struct.pack_into('<Q',m.map,HEADER.index('committed')*8,1)
    with self.assertRaises(RuntimeError):m.take()
    fields=[0]*21;fields[0]=1;fields[1]=2
    RECORD.pack_into(m.map,4096,*fields);m.take();self.assertEqual(len(m.rows),1)
   finally:m.close()
 def test_session_has_no_input_calls(self):
  source=(Path(__file__).parent/'session.py').read_text()
  for call in ('.button(','.move(','.activate(','.key(','.settle_pointer(', 'XTestFake','/dev/input'):
   self.assertNotIn(call,source)


class CleanupTests(unittest.TestCase):
 def test_bad_observer_does_not_skip_fixture_retirement(self):
  import tempfile
  from unittest.mock import Mock,patch
  import session
  rec=Mock(records=[],dropped=0);rec.close.side_effect=RuntimeError('detachment')
  m=Mock(rows=[]);m.read.return_value=1;fixture=Mock()
  raw=dict(drops={},cleanup={})
  with tempfile.TemporaryDirectory() as d,patch.object(session,'finish',return_value={'exit':0}) as retire:
   session.finalize(Path(d),raw,rec,None,None,m,fixture,True)
   retire.assert_called_once_with(fixture);m.write.assert_called_once_with('stop',1);m.close.assert_called_once()
   self.assertEqual(raw['error'],'observer_cleanup_incomplete')

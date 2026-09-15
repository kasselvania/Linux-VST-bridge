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


class SessionTests(unittest.TestCase):
 def ordered(self,rows):
  from session import FIELDS
  return [dict(dict.fromkeys(FIELDS,0),**r) | {'commit':i+1} for i,r in enumerate(rows)]
 def release(self,qpc=1020,dispatch=7):
  return [dict(kind=2,value=1,message=0x247,hwnd=73,qpc=qpc,end_qpc=qpc+1,message_time=1010),
          dict(kind=4,message=0x247,hwnd=73,qpc=qpc+2,dispatch=dispatch),
          dict(kind=5,message=0x247,hwnd=73,qpc=qpc+3,dispatch=dispatch),
          dict(kind=3,message=0x247,hwnd=73,qpc=qpc+2,end_qpc=qpc+4,dispatch=dispatch,message_time=1010)]
 def raw(self):
  # Independent Linux and fixture-owned Windows action boundaries. Contact IDs
  # are deliberately unrelated to Windows dispatch identity.
  return dict(schema=1,action_begin_ns=990000000,raw=[
   dict(kind='raw_touch_begin',device=1,source=2,detail=44,observed_ns=1005000000),
   dict(kind='raw_touch_end',device=1,source=2,detail=44,observed_ns=1011000000)],
   x11=[dict(kind='core_up',server_ms=1010,observed_ns=1011000000)],
   windows=self.ordered([dict(kind=7,qpc=1000,turn=2,flags=0,value=1),
                         dict(kind=5,qpc=1001,message=0x8732,value=1)]+self.release()),
   windows_clocks=[dict(linux_before_ns=1029000000,linux_after_ns=1031000000,windows_qpc=1030,tick=1030,frequency=1000)]*2,
   x_clocks=[dict(linux_before_ns=999000000,linux_after_ns=1001000000,server_ms=1000)]*2,
   status=dict(frequency=1000,calibration_ok=1,error=0),drops={},error=None,final_pointer=dict(mask=0))
 def test_full_summary_prompt(self):
  from session import summary
  s=summary(self.raw());self.assertEqual(s['disposition'],'UIR2_EMBEDDED_PRODUCT_ROUTE_REQUIRED')
  self.assertEqual(s['procedure_gaps_ms'],[1.0])
 def test_summary_missing_and_conflicting(self):
  from session import summary
  for mutate in (lambda r:r['raw'][1].update(source=3),lambda r:r.update(error='cleanup'),
                 lambda r:r['drops'].update(windows=1),lambda r:r['windows_clocks'].clear(),
                 lambda r:r['windows'][3].update(dispatch=0)):
   r=self.raw();mutate(r);self.assertEqual(summary(r)['disposition'],INSUFFICIENT)
 def test_mapping_commit_and_closed_mailbox(self):
  import tempfile,struct
  from session import Mapping,HEADER,SIZE,RECORD
  with tempfile.TemporaryDirectory() as d:
   p=Path(d)/'status'
   with p.open('wb') as f:f.truncate(SIZE)
   with p.open('r+b') as f:
    f.write(struct.pack('<3Q',0x32524955,2,SIZE))
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


class ActionBindingTests(unittest.TestCase):
 ordered=SessionTests.ordered
 release=SessionTests.release
 raw=SessionTests.raw
 def noisy(self,post=True):
  r=self.raw();r['windows']=self.ordered(self.release(994,3)+[
   dict(kind=1,qpc=999,end_qpc=999,queue=0x10001000),
   dict(kind=7,qpc=1000,turn=2,flags=0,value=1),
   dict(kind=5,qpc=1001,message=0x8732,value=1)]+(self.release() if post else []))
  return r
 def test_pre_arm_release_cannot_supply_post_arm_linux_release(self):
  from session import summary
  s=summary(self.noisy(False));self.assertEqual(s['disposition'],INSUFFICIENT)
  self.assertEqual(s['windows_release_removals'],0)
  self.assertEqual(s['message_timestamp_intervals_after_x_release_ms'],[])
 def test_pre_arm_noise_does_not_poison_exact_post_arm_chain(self):
  from session import summary
  s=summary(self.noisy());self.assertEqual(s['disposition'],'UIR2_EMBEDDED_PRODUCT_ROUTE_REQUIRED')
  self.assertEqual(s['windows_release_removals'],1)
  self.assertEqual(s['procedure_gaps_ms'],[1.0])
 def test_pre_arm_procedure_and_dispatch_cannot_complete_post_arm_chain(self):
  from session import summary
  for keep in ((2,),(2,3),(2,4),(2,3,4),(2,3,5)):
   r=self.noisy(False)
   r['windows']=self.ordered(r['windows']+[dict(row,commit=0) for row in self.release(dispatch=3) if row['kind'] in keep])
   self.assertEqual(summary(r)['disposition'],INSUFFICIENT)
 def test_absent_multiple_and_malformed_boundaries(self):
  from session import summary
  for change in ({'value':0},{'value':2},{'flags':1},{'qpc':0},{'qpc':True},{'dispatch':1},{'turn':0},{'end_qpc':1001}):
   r=self.raw();r['windows'][0].update(change)
   self.assertEqual(summary(r)['disposition'],INSUFFICIENT,change)
  for rows in (self.raw()['windows'][1:],self.raw()['windows'][:1]+self.raw()['windows']):
   r=self.raw();r['windows']=self.ordered(rows)
   self.assertEqual(summary(r)['disposition'],INSUFFICIENT)
 def test_reversed_boundary_cannot_authorize_rows(self):
  from session import summary
  r=self.raw();r['windows'][0]['qpc']=1050
  self.assertEqual(summary(r)['disposition'],INSUFFICIENT)
  r=self.noisy();r['windows'][0]['end_qpc']=1001
  self.assertEqual(summary(r)['disposition'],INSUFFICIENT)
 def test_pre_arm_queue_timestamp_and_detail_rows_are_excluded(self):
  from session import windows_action
  r=self.noisy();r['windows']=self.ordered([dict(kind=6,qpc=993,message=0x240,message_time=55)]+r['windows'])
  rows,boundary,error=windows_action(r['windows'])
  self.assertIsNone(error);self.assertEqual(boundary['qpc'],1000)
  self.assertFalse(any(row['kind'] in (1,6) for row in rows))
  self.assertTrue(all(row['qpc']>=1000 for row in rows))
 def test_pre_arm_clocks_and_calibration_do_not_authorize_action(self):
  from session import summary
  r=self.raw();r['windows_clocks'][0]['windows_qpc']=999
  self.assertEqual(summary(r)['disposition'],INSUFFICIENT)
  r=self.raw();r['windows']=self.ordered([dict(kind=5,qpc=998,message=0x8732,value=1)]+[row for row in r['windows'] if row['message']!=0x8732])
  self.assertEqual(summary(r)['disposition'],INSUFFICIENT)
 def test_retained_no_touch_shape_remains_insufficient(self):
  from session import summary
  r=self.raw();r['raw']=[];r['x11']=[];r['windows']=[]
  self.assertEqual(summary(r)['disposition'],INSUFFICIENT)

if __name__=='__main__':unittest.main()

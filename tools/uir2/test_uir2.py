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

import unittest,types
from unittest.mock import patch
import ap4_preview as owner
r=owner.runtime
class Cleanup(unittest.TestCase):
 def record(self,pid,start,group,command='endpoint'):
  return dict(pid=pid,start_ticks=start,pgrp=group,session=group,cmdline=command,comm='test')
 def root(self):
  return types.SimpleNamespace(pid=100,poll=lambda:0,wait=lambda **_:0)
 def test_unrelated_owner_stage_argument_and_healthy_sibling_are_not_owned(self):
  unrelated=[self.record(200,20,200,'owner --environment .wf0-factory-census.stage-test'),self.record(300,30,300,'healthy sibling')]
  with patch.object(r,'process_census',return_value=unrelated),patch.object(r.os,'killpg') as kill:
   self.assertEqual(r.cleanup_process(self.root(),[(100,10)]),dict(owned_descendants_zero=True,process_group_empty=True));kill.assert_not_called()
 def test_reused_pid_is_not_an_owned_identity(self):
  with patch.object(r,'process_census',return_value=[self.record(101,99,500)]),patch.object(r.os,'killpg') as kill:
   self.assertTrue(r.cleanup_process(self.root(),[(100,10),(101,11)])['owned_descendants_zero']);kill.assert_not_called()
 def test_surviving_owned_identity_or_process_group_is_failure(self):
  for record in [self.record(101,11,600),self.record(102,12,100)]:
   with self.subTest(record=record),patch.object(r,'process_census',return_value=[record]),patch.object(r,'CLEANUP_SECONDS',0),patch.object(r.os,'killpg'):
    with self.assertRaisesRegex(RuntimeError,'survived cleanup'):r.cleanup_process(self.root(),[(100,10),(101,11)])
if __name__=='__main__':unittest.main()

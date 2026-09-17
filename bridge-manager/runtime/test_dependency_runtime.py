"""Operation-container authority, with no Wine or proprietary application."""
import pathlib,tempfile,types,unittest
from unittest.mock import Mock,patch
import session as s

class RuntimeTests(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
  root=pathlib.Path(self.tmp.name).resolve()
  self.owner=types.SimpleNamespace(root=root,directory=root,op='a'*32,retiring=False,
   cancelled=lambda:False,ledger=Mock(),spec={'installer_launch':{'path':'/fixed/adapter.exe'},
   'application':{'environment':{'root':str(root),'runner':{'entry_point':'/fixed/runtime/_v2-entry-point',
   'proton':'/fixed/proton','files':[]}}}})
  self.runtime=s.Nad1Runtime(self.owner);self.addCleanup(self.runtime.close)
 def test_missing_or_duplicate_client_identity_refuses(self):
  artifact={'path':str(self.runtime.client),'sha256':'a'*64}
  for files in ([],[artifact,artifact]):
   self.runtime.runner['files']=files
   with self.assertRaisesRegex(ValueError,'client_identity'),patch.object(s,'verify') as verify:
    self.runtime.verify_tools()
   verify.assert_not_called()
 def test_client_and_service_bytes_are_verified(self):
  artifact={'path':str(self.runtime.client),'sha256':'a'*64};self.runtime.runner['files']=[artifact]
  with patch.object(s,'verify') as verify:self.runtime.verify_tools()
  self.assertEqual(verify.call_args_list[0].args[0],artifact)
  self.assertEqual(verify.call_args_list[1].args[0],{'path':str(self.runtime.service),'sha256':s.Nad1Runtime.SERVICE_SHA})
  with patch.object(s,'verify',side_effect=RuntimeError('changed')):
   with self.assertRaises(RuntimeError):self.runtime.verify_tools()
 def test_commands_reuse_container_and_only_forward_closed_diagnostics(self):
  r=self.runtime;r.socket=pathlib.Path('/fixed/private/socket');r.child=Mock(returncode=None)
  with patch.object(r,'start') as start,patch.object(r,'drain'),patch.object(r,'verify_tools'):
   a=r.argv(self.owner.root/'one.private',self.owner.root,{'EVIL':'value'})
   b=r.argv(self.owner.root/'two.private',self.owner.root/'home',{})
  start.assert_not_called();self.assertEqual(a[0],str(r.client));self.assertEqual(a[1],b[1])
  self.assertNotIn('EVIL',' '.join(a));self.assertNotIn('--clear-env',a)
  self.assertEqual([x for x in a if x.startswith('--pass-env=')],['--pass-env='+x for x in r.DEBUG_KEYS])
  self.assertEqual(a[-4:-1],['/fixed/proton','runinprefix','/fixed/adapter.exe'])
  self.assertIn('--directory='+str(self.owner.root/'home'),b)
 def test_dead_or_retired_container_never_falls_back_to_another_container(self):
  r=self.runtime;r.child=Mock(returncode=1)
  with patch.object(r,'drain'),patch.object(r,'start') as start:
   with self.assertRaisesRegex(ValueError,'runtime_exited'):r.argv('/request','/cwd',{})
  start.assert_not_called();r.close()
  with self.assertRaisesRegex(ValueError,'runtime_retired'):r.argv('/request','/cwd',{})
 def test_single_shot_retirement_closes_runtime_even_when_scm_refuses(self):
  owner=s.Nad1Owner({'operation':'a'*32,'report':'/unused/result.json','application':{'environment':{'root':'/unused'}}},None,None,lambda:False)
  owner.runtime=Mock()
  with patch.object(owner,'_retire_service',return_value=False):self.assertFalse(owner.retire())
  owner.runtime.close.assert_called_once()

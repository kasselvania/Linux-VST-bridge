"""Opaque return delivery and privacy, with fake bytes and no browser account."""
import os,pathlib,socket,struct,tempfile,time,types,unittest
from unittest.mock import Mock
import session as s
class UriTests(unittest.TestCase):
 def test_closed_uri(self):
  for good in [b'native-access:fixture',b'native-access://auth?code=fixture%2b&state=generated']:
   self.assertTrue(s.native_access_uri(good))
  for bad in [b'https://example.test',b'native-access:',b'native-access:x --no-sandbox',b'native-access:x"',b'native-access:x\\',b'native-access:x\n',b'native-access:%00',b'native-access:%7f',b'native-access:%',b'native-access:%abz\x00',b'native-access:'+b'a'*2048]:
   self.assertFalse(s.native_access_uri(bad),bad)
@unittest.skipUnless(hasattr(socket,'SO_PEERCRED'),'Linux peer credentials')
class CallbackTests(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
  self.read,self.write=os.pipe();self.addCleanup(os.close,self.read)
  self.child=types.SimpleNamespace(returncode=None,stdin=os.fdopen(self.write,'wb',buffering=0));self.addCleanup(self.child.stdin.close)
  self.cut=Mock();self.op='a'*32;self.address=str(pathlib.Path(self.tmp.name)/'callback')
  self.b=s.RendererCallback(self.op,self.child,self.cut,address=self.address);self.addCleanup(self.b.close)
 def connect(self,op=None):
  peer=socket.socket(socket.AF_UNIX);peer.connect(self.address);peer.settimeout(1);self.addCleanup(peer.close)
  peer.sendall((op or self.op).encode());self.b.tick();return peer
 def test_wrong_operation_has_no_delivery_or_privacy_change(self):
  p=self.connect('b'*32);self.assertEqual(p.recv(4),b'\x01');self.cut.assert_not_called();self.assertEqual(self.b.attempts,0)
 def test_exact_payload_in_pipe_only_and_ack_is_not_authentication(self):
  p=self.connect();self.assertEqual(p.recv(4),b'NAC1');uri=b'native-access:generated-secret'
  p.sendall(struct.pack('<I',len(uri))+uri);self.b.tick();self.cut.assert_called_once()
  self.assertEqual(os.read(self.read,4096),struct.pack('<I',len(uri))+uri)
  self.assertEqual(self.b.pending,b'');self.assertNotIn('generated-secret',str(self.b.value()))
  self.b.feed_ack(b'private unrelated output generated-secret\nNA_AUTH_V1 '+self.op.encode()+b' 1 0\n')
  self.assertEqual(p.recv(1),b'\0');self.assertEqual(self.b.value()['dispatched'],1)
  self.assertEqual(self.b.value()['authentication'],'not_observed')
 def test_malformed_truncated_oversized_and_timeout_refuse(self):
  p=self.connect();p.recv(4);p.sendall(struct.pack('<I',3000));self.b.tick();self.assertEqual(p.recv(1),b'\x01');self.cut.assert_not_called()
  p=self.connect();p.recv(4);p.sendall(struct.pack('<I',30)+b'native-access:');self.b.tick();self.b.deadline=0;self.b.tick();self.assertEqual(p.recv(1),b'\x02');self.cut.assert_not_called()
 def test_no_ack_retry_and_late_wrong_ack_not_accepted(self):
  p=self.connect();p.recv(4);uri=b'native-access:fixture';p.sendall(struct.pack('<I',len(uri))+uri);self.b.tick();os.read(self.read,4096)
  self.b.feed_ack(b'NA_AUTH_V1 '+self.op.encode()+b' 2 0\n');self.assertEqual(self.b.delivered,0)
  self.b.deadline=0;self.b.tick();self.assertEqual(p.recv(1),b'\x02');self.assertEqual(self.b.attempts,1)
 def test_primary_exit_revokes_socket_without_launch(self):
  p=self.connect();p.recv(4);self.child.returncode=0;self.b.tick();self.assertTrue(self.b.closed);self.cut.assert_not_called()

class RuntimePrivacyTests(unittest.TestCase):
 def test_shared_runtime_does_not_retain_output_after_private_return(self):
  from unittest.mock import patch
  runtime=object.__new__(s.Nad1Runtime)
  runtime.owner=types.SimpleNamespace(diagnostic_privacy=True)
  runtime.capture=Mock();runtime.output=bytearray();runtime.sel=Mock()
  runtime.sel.select.return_value=[(types.SimpleNamespace(fileobj=types.SimpleNamespace(fileno=lambda:4),data='stderr'),None)]
  with patch.object(s.os,'read',return_value=b'generated-return-code'):
   runtime.drain()
  runtime.capture.write.assert_not_called()
  runtime.owner.diagnostic_privacy=False
  with patch.object(s.os,'read',return_value=b'ordinary diagnostics'):
   runtime.drain()
  runtime.capture.write.assert_called_once_with(b'ordinary diagnostics')

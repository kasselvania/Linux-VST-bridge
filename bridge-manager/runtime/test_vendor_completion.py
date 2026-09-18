"""Arturia's ordinary completion must not call a renderer-only local helper."""
import hashlib,json,os,pathlib,signal,subprocess,sys,tempfile,unittest
from unittest.mock import Mock,patch
import session as s
class VendorCompletion(unittest.TestCase):
 def test_clean_companion_exit_reports_completed(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=pathlib.Path(tmp).resolve();image=root/'fixture';image.write_bytes(b'generated')
   artifact={'path':str(image),'sha256':hashlib.sha256(image.read_bytes()).hexdigest()}
   app={'executable':artifact,'helpers':[],'environment':{'root':str(root),'runner':{'files':[]}}}
   scope=Mock();scope.members.return_value=[];scope.group='generated-owned-group'
   library=Mock();library.prctl.return_value=0
   signals=[signal.getsignal(x) for x in (signal.SIGTERM,signal.SIGINT)]
   try:
    with patch.object(s,'CompanionCgroup',return_value=scope),patch.object(s.ctypes,'CDLL',return_value=library),patch.object(s.os,'waitid',return_value=None),patch.object(s,'vendor_launch',return_value=([sys.executable,'-c','pass'],root)),patch.object(s,'environment',return_value=os.environ.copy()):
     self.assertTrue(s.vendor_application({'application':app,'report':str(root/'result.json')}))
    r=json.loads((root/'result.json').read_bytes());self.assertEqual(r['state'],'completed');self.assertIsNone(r['error']);self.assertEqual(r['launcher_exit'],0);self.assertTrue(r['cleanup_confirmed'])
   finally:
    for sig,old in zip((signal.SIGTERM,signal.SIGINT),signals):signal.signal(sig,old)

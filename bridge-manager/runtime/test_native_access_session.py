"""NAO1 closed admission and result law; no vendor application or service."""
import copy,hashlib,json,pathlib,sys,tempfile,unittest
from unittest.mock import patch
sys.path.insert(0,str(pathlib.Path(__file__).parent))
import ownership
import session as s

def canonical(value):
 return json.dumps(value,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()

class SessionAdmission(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup);self.home=pathlib.Path(self.tmp.name).resolve()
  self.managed=self.home/'.local/share/linux-vst-bridge/managed';self.directory=self.managed/'vendor-applications/native-access-dependency';self.op='a'*32
  self.operation=self.directory/'operations'/self.op;self.operation.mkdir(parents=True)
  self.root=self.managed/'environments'/('e'*32);self.prefix=self.root/'compatdata/pfx';self.prefix.mkdir(parents=True)
  self.app={'schema':1,'id':'native-access','environment':{'id':'e'*32,'root':str(self.root)},'files':{},
   'installation':{'path':str(self.managed/'installation.json'),'sha256':'1'*64},'observation_sha256':'2'*64,'source_seal_sha256':'3'*64}
  self.appid=hashlib.sha256(canonical(self.app)).hexdigest();self.software={'manager':{'sha256':'4'*64}};self.software_sha=hashlib.sha256(canonical(self.software)).hexdigest()
  self.daemon={'sha256':'5'*64,'size':18259440,'architecture':'x64'}
  prior={'schema':2,'kind':'native_access_dependency','operation':self.op,'application':self.app,'application_identity':self.appid,
   'software':self.software,'software_sha256':self.software_sha}
  s.installer_atomic(self.operation/'spec.json',prior);spec_sha=hashlib.sha256((self.operation/'spec.json').read_bytes()).hexdigest()
  root_record={'frame':['NAD1_INSTALL_ROOT_V1',self.op,'6'*64,s.NAD1_INSTALLER_SHA,'35769456','10','20']}
  s.installer_atomic(self.operation/(self.op+'-installer-root.private.json'),root_record);root_sha=hashlib.sha256((self.operation/(self.op+'-installer-root.private.json')).read_bytes()).hexdigest()
  record={'schema':1,'operation':self.op,'application':self.appid,'software_sha256':self.software_sha,
   'installer_sha256':s.NAD1_INSTALLER_SHA,'daemon':self.daemon,'root_sha256':root_sha,'spec_sha256':spec_sha}
  s.installer_atomic(self.operation/'artifact.json',record);s.installer_atomic(self.directory/'artifact.json',record)
  artifact_sha=hashlib.sha256((self.directory/'artifact.json').read_bytes()).hexdigest()
  prefix_md=self.prefix.lstat()
  self.session={'schema':1,'authority':'retained_exact_installation_artifact_and_current_software','application':self.appid,
   'software_sha256':self.software_sha,'manager_sha256':'4'*64,'environment':'e'*32,
   'prefix':{'path_sha256':hashlib.sha256(bytes(str(self.prefix),'utf-8')).hexdigest(),
    'device':prefix_md.st_dev,'inode':prefix_md.st_ino},'installation':self.app['installation'],
   'service':s.NAD1_SERVICE,'listeners':[5146,5563],'installer':{'sha256':s.NAD1_INSTALLER_SHA,'size':35769456},
   'daemon':self.daemon,'origin':{'operation':self.op,'artifact_sha256':artifact_sha,'spec_sha256':spec_sha,
    'root_sha256':root_sha,'software_sha256':self.software_sha},'current_software':self.software}
  self.spec={'operation':'b'*32,'application':self.app,'application_identity':self.appid,'software':self.software,
   'software_sha256':self.software_sha,'dependency_session':self.session}
 def admit(self,spec=None):
  with patch.object(pathlib.Path,'home',return_value=self.home),patch.object(s,'nad1_recovery',return_value={'daemon':self.daemon}),patch.object(ownership,'image_identity',return_value=self.daemon):
   return s.nad1_session_admitted(spec or self.spec)
 def test_exact_artifact_and_current_software_admit_without_prepared_receipt(self):
  self.assertEqual(self.admit()['daemon'],self.daemon)
  self.assertFalse((self.directory/'prepared.json').exists())
 def test_wrong_application_software_installer_or_origin_refuses(self):
  cases=[]
  for path,value in [(('application',),'f'*64),(('software_sha256',),'f'*64),(('manager_sha256',),'f'*64),
                     (('installer','sha256'),'f'*64),(('service',),'OtherService'),(('origin','operation'),'c'*32)]:
   bad=copy.deepcopy(self.spec);target=bad['dependency_session']
   for key in path[:-1]:target=target[key]
   target[path[-1]]=value;cases.append(bad)
  for bad in cases:
   with self.subTest(bad=bad['dependency_session']):
    with self.assertRaises(ValueError):self.admit(bad)
 def test_changed_missing_malformed_or_aliased_artifact_refuses(self):
  pointer=self.directory/'artifact.json';original=pointer.read_bytes()
  for content in (b'{}',b'{"schema":1,"schema":2}',b'changed'):
   pointer.write_bytes(content)
   with self.assertRaises((ValueError,json.JSONDecodeError)):self.admit()
   pointer.write_bytes(original)
  pointer.unlink();pointer.symlink_to(self.operation/'artifact.json')
  with self.assertRaises(ValueError):self.admit()
 def test_replaced_or_aliased_prefix_refuses(self):
  original=self.prefix.rename(self.root/'old-prefix');self.prefix.symlink_to(original)
  with self.assertRaises(ValueError):self.admit()
 def test_renderer_launch_uses_session_admission_not_prepared_receipt(self):
  with patch.object(s,'renderer_validate'),patch.object(s,'nad1_session_admitted',return_value=self.session) as admit,patch.object(s,'nad1_prepared',side_effect=AssertionError('historical gate used')),patch.object(s,'renderer_owned',return_value=True) as run:
   self.assertTrue(s.renderer_application(self.spec));admit.assert_called_once_with(self.spec);run.assert_called_once()
 def test_application_failure_and_operator_stop_remain_distinct_terminal_states(self):
  self.assertEqual(s.renderer_terminal_state(True,False,None),'completed')
  self.assertEqual(s.renderer_terminal_state(True,True,None),'cancelled')
  self.assertEqual(s.renderer_terminal_state(True,False,'application_outer_nonzero'),'failed')
  self.assertEqual(s.renderer_terminal_state(False,True,None),'failed')

if __name__=='__main__':unittest.main()

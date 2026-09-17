import copy,importlib.util,json,os,pathlib,sys,tempfile,unittest
HERE=pathlib.Path(__file__).resolve().parent
sys.path.insert(0,str(HERE));sys.path.insert(1,str(HERE.parent/'naui1'))
from classify import classify
from common import decode,Refusal,read,file_identity,publish
from census import registry,bundle
from package import verify
class ObserverTests(unittest.TestCase):
 def state(self):return dict(schema=1,complete=True,installer='exact',daemon='exact',registration='exact',service='running',process='exact',readiness='exact',foreign='absent')
 def test_state_law(self):
  for changes,want in [({},'READY'),({'daemon':'absent','registration':'absent','service':'absent','process':'absent','readiness':'unavailable'},'ABSENT'),({'registration':'absent'},'UNREGISTERED'),({'service':'stopped','process':'absent','readiness':'unavailable'},'STOPPED'),({'readiness':'not_ready'},'RUNNING_NOT_READY'),({'foreign':'exact'},'CONFLICT')]:
   v=self.state();v.update(changes);expected={'READY':'NAD1_DEPENDENCY_READY','ABSENT':'NAD1_DEPENDENCY_ABSENT','CONFLICT':'NAD1_FOREIGN_DAEMON_CONFLICT'}.get(want,'NAD1_SERVICE_'+want);self.assertEqual(classify(v),expected)
 def test_false_readiness_and_losses(self):
  for changes in [{'process':'absent'},{'complete':False},{'foreign':'unavailable'},{'registration':'unavailable'},{'daemon':'ambiguous'},{'installer':'ambiguous'},{'service':'unavailable'}]:
   v=self.state();v.update(changes);self.assertEqual(classify(v),'NAD1_IDENTITY_UNRESOLVED')
 def test_schema_mutations(self):
  for k in self.state():
   v=self.state();del v[k]
   with self.assertRaises(ValueError):classify(v)
   v=self.state();v[k]=[]
   with self.assertRaises(ValueError):classify(v)
  v=self.state();v['command']='secret'
  with self.assertRaises(ValueError):classify(v)
 def test_duplicate_json_and_no_replace(self):
  with self.assertRaises(Refusal):decode(b'{"x":1,"x":2}')
  with tempfile.TemporaryDirectory() as d:
   p=pathlib.Path(d).resolve()/'result';publish(p,{'x':1})
   with self.assertRaises(FileExistsError):publish(p,{'x':2})
   self.assertEqual(decode(read(p)),{'x':1})
 def test_aliases_and_changed_bytes(self):
  with tempfile.TemporaryDirectory() as d:
   root=pathlib.Path(d).resolve();p=root/'a';p.write_bytes(b'original');p.chmod(0o600);h=file_identity(p)['sha256'];p.write_bytes(b'changed')
   with self.assertRaises(Refusal):read(p,expected=h)
   q=root/'link';q.symlink_to(p)
   with self.assertRaises(OSError):read(q)
   q.unlink();os.link(p,q)
   with self.assertRaises(Refusal):read(p)
 def test_registry_exact_and_duplicate(self):
  head=b'WINE REGISTRY Version 2\n';body=b'[System\\\\CurrentControlSet\\\\Services\\\\NTKDaemonService]\n"Start"=dword:00000002\n'
  self.assertEqual(len(registry(head+body)),1);self.assertEqual(registry(head),[])
  with self.assertRaises(Refusal):registry(head+body+body)
  with self.assertRaises(Refusal):registry(b'partial')
 def test_ambiguous_installers(self):
  with tempfile.TemporaryDirectory() as d:
   drive=pathlib.Path(d).resolve();p=drive/'Program Files/Native Instruments/Native Access/resources/daemon/win';p.mkdir(parents=True)
   (p/'NTKDaemon 1 Setup PC.exe').touch();(p/'NTKDaemon 2 Setup PC.exe').touch()
   with self.assertRaises(Refusal):bundle(drive)
 def test_seal_missing_required_files(self):
  with tempfile.TemporaryDirectory() as d:
   with self.assertRaises(Refusal):verify(d)
if __name__=='__main__':unittest.main()

class AttributionTests(unittest.TestCase):
 def test_numbered_registry_controlset_is_presence_not_absence(self):
  head=b'WINE REGISTRY Version 2\n'
  for name in ('CurrentControlSet','ControlSet001','ControlSet002','cOnTrOlSeT003'):
   block=('['+'System\\\\'+name+'\\\\Services\\\\NTKDaemonService'+']\n"Type"=dword:00000010\n').encode()
   self.assertEqual(len(registry(head+block)),1)
  for name in ('ControlSet1','OtherControlSet001'):
   block=('['+'System\\\\'+name+'\\\\Services\\\\NTKDaemonService'+']\n').encode()
   self.assertEqual(registry(head+block),[])
  first=b'[System\\\\ControlSet001\\\\Services\\\\NTKDaemonService]\n'
  second=first.replace(b'001',b'002')
  with self.assertRaises(Refusal):registry(head+first+second)
 def test_reference_is_not_unique_failure_authority(self):
  from observer import evidence
  for op in ['aa'*16,'bb'*16]:
   r={'operation':op,'cleanup_confirmed':True,'owned_live':0,'state':'completed','application_identity':'7228a542c01b89daa5d04b9c8566af235ee7a2358e52f741b8918239c3a7d26e','renderer':{'launch_binding':{'status':'bound'},'windows_dropped_observations':0},'diagnostics':{},'requested':'inherited','effective':{}}
   v=evidence(op,json.dumps(r).encode(),b'CreateProcessInternalW helper NTKDaemon 1.32.0 Setup PC.exe\n',json.dumps({'operation':op}).encode())
   self.assertEqual(v['facts'][0]['class'],'bundled_daemon_installer_reference_in_create_record')
   self.assertEqual(v['operation'],op)
 def test_foreign_and_deleted_prefix_never_ready(self):
  v=ObserverTests().state();v['foreign']='exact'
  self.assertEqual(classify(v),'NAD1_FOREIGN_DAEMON_CONFLICT')
 def test_listener_or_command_exit_does_not_create_ready(self):
  v=ObserverTests().state()
  for field in ['listener','helper_exit','service_exit','pid','command','raw_log','token']:
   bad=copy.deepcopy(v);bad[field]=0
   with self.assertRaises(ValueError):classify(bad)
 def test_process_without_readiness(self):
  v=ObserverTests().state();v['readiness']='unavailable'
  self.assertEqual(classify(v),'NAD1_SERVICE_RUNNING_NOT_READY')
 def test_missing_installer_is_absence_only_if_census_complete(self):
  v=ObserverTests().state();v.update(installer='absent',daemon='absent',registration='absent',service='absent',process='absent',readiness='unavailable')
  self.assertEqual(classify(v),'NAD1_DEPENDENCY_ABSENT');v['complete']=False
  self.assertEqual(classify(v),'NAD1_IDENTITY_UNRESOLVED')

class PEMetadataTests(unittest.TestCase):
 def executable(self, architecture=0x14c):
  import struct
  data=bytearray(512);data[:2]=b'MZ';struct.pack_into('<I',data,60,64)
  data[64:68]=b'PE\0\0';struct.pack_into('<HH',data,68,architecture,1)
  struct.pack_into('<HH',data,84,224,2)
  struct.pack_into('<H',data,88,0x10b if architecture==0x14c else 0x20b)
  return data
 def test_optional_version_refusal_retains_architecture_not_strings(self):
  from unittest.mock import patch
  from pe_metadata import metadata
  for machine,want in [(0x14c,'x86'),(0x8664,'x64')]:
   with tempfile.TemporaryFile() as f:
    f.write(self.executable(machine));f.flush()
    with patch('pe_metadata.pe',side_effect=Refusal('version_string_type')):
     self.assertEqual(metadata(f),dict(architecture=want,version={},version_status='unavailable',version_reason='version_string_type'))
 def test_structural_or_architecture_failure_is_not_optional(self):
  from unittest.mock import patch
  from pe_metadata import metadata
  with tempfile.TemporaryFile() as f:
   f.write(self.executable());f.flush()
   for code in ['metadata_truncated','pe_resource_mapping','version_duplicate']:
    with patch('pe_metadata.pe',side_effect=Refusal(code)),self.assertRaises(Refusal):metadata(f)
   f.seek(88);f.write(b'\x0b\x02');f.flush()
   with self.assertRaises(Refusal):metadata(f)
 def test_absent_version_is_distinct_from_unreadable(self):
  from pe_metadata import metadata
  with tempfile.TemporaryFile() as f:
   f.write(self.executable());f.flush()
   self.assertEqual(metadata(f),dict(architecture='x86',version={},version_status='absent',version_reason=None))

class RetainedResultTests(unittest.TestCase):
 def test_completed_observation_stays_unresolved_and_sealed(self):
  import hashlib
  root=HERE.parents[1]/'evidence/nad1/observation/completed'
  raw=(root/'observation.json').read_bytes();seal=(root/'source-seal.json').read_bytes()
  self.assertEqual(hashlib.sha256(raw).hexdigest(),'3d14b33cf14fef1528cc7ac594fcd3707f262a227280f25b9f991938bdd0d5b2')
  self.assertEqual(hashlib.sha256(seal).hexdigest(),'3989f31bb9d3924b47d2a8ffd5730a6bf1c43578db974d1c942e2dc9f2743212')
  r=decode(raw);self.assertEqual(classify(r['state']),r['disposition'])
  self.assertEqual(r['disposition'],'NAD1_IDENTITY_UNRESOLVED')
  self.assertEqual(r['process_census']['unavailable'],5)
  self.assertFalse(r['state']['complete'])
  self.assertEqual(r['dependency_failure_snapshot_operation'],'354af73fea5773244ac3ebd21425e4ed')
  for forbidden in [b'/home/deck',b'http://',b'https://',b'"pid"',b'"cmdline"',b'"environ"']:
   self.assertNotIn(forbidden,raw)
 def test_loss_cannot_be_overridden_by_absent_files_or_service(self):
  r=decode((HERE.parents[1]/'evidence/nad1/observation/completed/observation.json').read_bytes())
  for change in [{'service':'stopped'},{'readiness':'exact'},{'daemon':'exact'},{'foreign':'absent'},{'complete':True}]:
   state=dict(r['state'],**change)
   self.assertEqual(classify(state),'NAD1_IDENTITY_UNRESOLVED')

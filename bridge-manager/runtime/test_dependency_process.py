import pathlib,struct,tempfile,unittest
from unittest.mock import patch
from dependency_process import census,bounded,image_identity,LIMITS,Extent

class CensusTests(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
  self.root=pathlib.Path(self.tmp.name).resolve();self.proc=self.root/'proc';self.proc.mkdir()
  self.prefix=self.root/'pfx';self.prefix.mkdir();self.image=self.prefix/'NTKDaemon.exe'
  data=bytearray(256);data[:2]=b'MZ';struct.pack_into('<I',data,60,64);data[64:68]=b'PE\0\0';data[68:70]=b'\x64\x86'
  self.image.write_bytes(data);self.artifact=image_identity(self.image)
 def task(self,name='NTKDaemon.exe',prefix=None):
  p=self.proc/'123';p.mkdir();self.stat(p,name,10)
  (p/'comm').write_bytes(name.encode()+b'\n')
  (p/'cmdline').write_bytes(str(self.image).encode()+b'\0')
  (p/'maps').write_bytes(b'1000-2000 r-xp 0 00:00 1 '+str(self.image).encode()+b'\n')
  (p/'environ').write_bytes(b'WINEPREFIX='+str(prefix or self.prefix).encode()+b'\0')
  return p
 def stat(self,p,name,ticks):
  (p/'stat').write_text('123 ('+name+') S '+'0 '*18+str(ticks)+'\n')
 def run_census(self,reader=bounded,admitted=True):return census(self.prefix,self.artifact if admitted else None,self.proc,reader=reader)
 def test_unrelated_unreadable_details_do_not_weaken(self):
  p=self.task('unrelated');(p/'cmdline').unlink();(p/'maps').unlink()
  r=self.run_census();self.assertEqual(r['unavailable'],0);self.assertEqual(r['candidates'],[]);self.assertEqual(r['unled_unreadable'],1)
 def test_comm_lead_unreadable_maps_is_ambiguity(self):
  p=self.task();(p/'maps').unlink();r=self.run_census()
  self.assertEqual(r['unavailable'],1);self.assertEqual(r['failure_counts'],{'maps_unavailable':1})
  self.assertIn('linux_comm_exact',r['candidates'][0]['lead_authorities'])
 def test_same_foreign_deleted_prefix_relation(self):
  p=self.task()
  for relation in ['same','foreign','deleted']:
   q=self.prefix if relation=='same' else self.root/relation
   if relation=='foreign':q.mkdir()
   (p/'environ').write_bytes(b'WINEPREFIX='+str(q).encode()+b'\0')
   r=self.run_census();self.assertEqual(r['unavailable'],0);self.assertTrue(r['candidates'][0]['exact']);self.assertEqual(r['candidates'][0]['prefix_relation'],relation)
 def test_pid_reuse_is_ambiguity(self):
  p=self.task();calls=0
  def read(path,kind):
   nonlocal calls
   if kind=='stat':
    calls+=1
    if calls==2:self.stat(p,'NTKDaemon.exe',11)
   return bounded(path,kind)
  r=self.run_census(read);self.assertEqual(r['failure_counts'],{'process_reused':1});self.assertFalse(r['candidates'][0]['exact'])
 def test_command_prose_is_not_process_authority(self):
  p=self.task('helper');(p/'cmdline').write_bytes(b'helper\0Please run NTKDaemon.exe next\0');(p/'maps').write_bytes(b'')
  r=self.run_census();self.assertEqual(r['candidates'],[]);self.assertEqual(r['unavailable'],0)
 def test_exact_command_argument_is_only_lead(self):
  p=self.task('helper');(p/'maps').write_bytes(b'')
  r=self.run_census();self.assertFalse(r['candidates'][0]['exact']);self.assertEqual(r['failure_counts'],{'image_unavailable':1})
 def test_maps_after_early_chunk_is_found(self):
  p=self.task('wine');(p/'cmdline').write_bytes(b'wine\0')
  original=(p/'maps').read_bytes();(p/'maps').write_bytes(b'0000-1000 r--p 0 00:00 0 /other\n'*500+original)
  r=self.run_census();self.assertTrue(r['candidates'][0]['exact']);self.assertEqual(r['candidates'][0]['lead_authorities'],['mapped_pe_basename_exact'])
 def test_oversized_command_maps_environment(self):
  p=self.task()
  for kind in ['cmdline','maps','environ']:
   original=(p/kind).read_bytes();(p/kind).write_bytes(b'x'*(LIMITS[kind][0]+1))
   r=self.run_census();self.assertEqual(r['failure_counts'],{'extent_exhausted':1});(p/kind).write_bytes(original)
 def test_record_count_exhaustion(self):
  p=self.task();(p/'environ').write_bytes(b'X=1\0'*8193)
  self.assertEqual(self.run_census()['failure_counts'],{'extent_exhausted':1})
 def test_unled_search_exhaustion_is_not_silent_absence(self):
  p=self.task('unrelated');(p/'cmdline').write_bytes(b'x'*(65536+1));(p/'maps').write_bytes(b'')
  r=self.run_census();self.assertEqual(r['search_extent_exhausted'],1);self.assertEqual(r['unavailable'],1)
 def test_no_leads_complete_census(self):
  r=self.run_census();self.assertEqual(r['candidates'],[]);self.assertEqual(r['unavailable'],0)
 def test_admitted_artifact_not_version_or_basename(self):
  self.task();r=self.run_census(admitted=False)
  self.assertEqual(r['failure_counts'],{'image_unavailable':1});self.assertFalse(r['candidates'][0]['exact'])
  self.artifact['sha256']='0'*64;self.assertEqual(self.run_census()['unavailable'],1)
 def test_environment_missing_duplicate_or_alias_refuses(self):
  p=self.task()
  for raw in [b'',b'WINEPREFIX=/p\0WINEPREFIX=/q\0',b'WINEPREFIX=relative\0']:
   (p/'environ').write_bytes(raw);self.assertEqual(self.run_census()['failure_counts'],{'prefix_relation_unavailable':1})
 def test_private_failure_and_public_projection(self):
  p=self.task();(p/'maps').unlink();r=self.run_census();private=r.pop('private')
  self.assertEqual(private[0]['linux_pid'],123);self.assertEqual(private[0]['start_ticks'],10)
  import json
  raw=json.dumps(r)
  for secret in [str(self.root),'linux_pid','start_ticks','WINEPREFIX']:
   self.assertNotIn(secret,raw)
 def test_descriptor_read_is_bounded(self):
  p=self.task();(p/'cmdline').write_bytes(b'x'*70000)
  lengths=[];import os
  original=os.read
  def read(fd,n):lengths.append(n);return original(fd,n)
  with patch('dependency_process.os.read',side_effect=read),self.assertRaises(Extent):bounded(p/'cmdline','cmdline')
  self.assertLessEqual(sum(lengths),65537)
if __name__=='__main__':unittest.main()

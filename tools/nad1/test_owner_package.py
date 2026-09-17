import importlib.util,pathlib,tempfile,unittest,os
import owner_package as p
class Package(unittest.TestCase):
 def fixture(self,d):
  d=pathlib.Path(d)
  for n in p.REQUIRED:(d/n).write_bytes(n.encode());(d/n).chmod(0o400)
  m={'schema':1,'source':{'head':'a'*40,'tree':'b'*40},'files':{n:p.digest(d/n) for n in p.REQUIRED}}
  p.publish(d/'seal.json',m);return d,p.digest(d/'seal.json')
 def test_every_input_is_sealed_and_no_replacement(self):
  with tempfile.TemporaryDirectory() as tmp:
   d,seal=self.fixture(tmp);p.verify(d,seal)
   for name in p.REQUIRED:
    f=d/name;f.chmod(0o600);saved=f.read_bytes();f.write_bytes(b'changed');f.chmod(0o400)
    with self.assertRaises(ValueError):p.verify(d,seal)
    f.chmod(0o600);f.write_bytes(saved);f.chmod(0o400)
   with self.assertRaises(FileExistsError):p.publish(d/'seal.json',{})
 def test_missing_extra_symlink_and_reduced_set_refuse(self):
  for case in ['missing','extra','symlink','reduced']:
   with self.subTest(case=case),tempfile.TemporaryDirectory() as tmp:
    d,seal=self.fixture(tmp);f=d/'session.py'
    if case=='extra':(d/'extra').write_text('x')
    else:f.unlink()
    if case=='symlink':f.symlink_to(d/'ownership.py')
    if case=='reduced':
     m=p.read(d/'seal.json');del m['files']['session.py'];(d/'seal.json').unlink();p.publish(d/'seal.json',m);seal=p.digest(d/'seal.json')
    with self.assertRaises(ValueError):p.verify(d,seal)
if __name__=='__main__':unittest.main()

class Preservation(unittest.TestCase):
 def test_only_exact_hardlinked_runtime_ctime_is_noncontent_change(self):
  from owner_campaign import compare_preservation
  import copy
  a={'system':{'active':True},'real_prefix_metadata_sha256':'a','environment_metadata':{'runtime-var/file':[0o100644,4,10,20,30,40],'compatdata/pfx/system.reg':[0o100644,4,10,20,30,41]},'runtime_content':{'runtime-var/file':{'sha256':'b'*64,'links':48}}}
  b=copy.deepcopy(a);b['environment_metadata']['runtime-var/file'][3]+=1;b['real_prefix_metadata_sha256']='c'
  self.assertEqual(compare_preservation(a,b)['runtime_shared_inode_ctime_changes'],1)
  for case in ['registry','content','mode','inode','mtime','links']:
   c=copy.deepcopy(b)
   if case=='registry':c['environment_metadata']['compatdata/pfx/system.reg'][3]+=1
   elif case=='content':c['runtime_content']['runtime-var/file']['sha256']='d'*64
   elif case=='links':c['runtime_content']['runtime-var/file']['links']=1
   else:c['environment_metadata']['runtime-var/file'][{'mode':0,'inode':5,'mtime':2}[case]]+=1
   with self.assertRaises(ValueError):compare_preservation(a,c)

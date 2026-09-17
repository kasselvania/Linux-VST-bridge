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

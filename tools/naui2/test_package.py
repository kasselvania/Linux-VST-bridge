import copy,hashlib,pathlib,tempfile,unittest
from unittest.mock import patch
import package as p
from campaign import compare

class Tests(unittest.TestCase):
    def test_closed_package_and_every_input_is_verified(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp)
            for name in p.REQUIRED:(root/name).write_bytes(b'generated')
            with (root/'payload.exe').open('ab') as f:f.truncate(67*1024*1024)
            m={'schema':1,'source':{'head':'a'*40,'tree':'b'*40},'files':{n:p.digest(root/n) for n in p.REQUIRED},'recipe':{'name':'generated-test'},'payload_size':67*1024*1024}
            for f in root.iterdir():f.chmod(0o400)
            p.publish(root/'manifest.json',m);seal=p.digest(root/'manifest.json');p.verify(root,seal)
            for n in p.REQUIRED:
                f=root/n;f.chmod(0o600)
                with self.assertRaises(ValueError):p.verify(root,seal)
                f.chmod(0o400)
            (root/'extra').touch()
            with self.assertRaises(ValueError):p.verify(root,seal)
            (root/'extra').unlink()
            with self.assertRaises(FileExistsError):p.publish(root/'manifest.json',m)
            with self.assertRaises(ValueError):p.verify(root,'f'*64)
    def test_reduced_required_set_and_symlink_and_duplicate_refuse(self):
        with self.assertRaises(ValueError):p.unique([('a',1),('a',2)])
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp);m={'schema':1,'source':{},'files':{},'recipe':{},'payload_size':67*1024*1024}
            p.publish(root/'manifest.json',m)
            with self.assertRaises(ValueError):p.verify(root,p.digest(root/'manifest.json'))
    def test_comparison_rejects_identity_order_and_failure_mutations(self):
        identity={'seal':'a'*64,'source':{'head':'b'*40},'files':{'session.py':'c'*64}}
        rows=[{'mode':mode,'operation':str(i)*32,'identity':copy.deepcopy(identity),'cleanup_confirmed':True,'owned_live':0,'outer_exit':0,'prefix_removed':True,'effective':{'policy':mode},'renderer':{'complete':True,'cause':cause,'launch_binding':{'status':'bound'}}} for i,(mode,cause) in enumerate([('inherited','gpu'),('software_rendering','unresolved'),('inherited','gpu')])]
        self.assertEqual(compare(rows,identity),'NAUI2_GENERATED_PRODUCTION_DIFFERENTIAL_PASSED')
        for field,value in [('cleanup_confirmed',False),('owned_live',1),('outer_exit',3),('prefix_removed',False),('identity',{}),('mode','inherited'),('operation','0'*32)]:
            changed=copy.deepcopy(rows);changed[1][field]=value
            with self.assertRaises(ValueError):compare(changed,identity)
        for field,value in [('complete',False),('cause','gpu'),('launch_binding',{'status':'unknown'})]:
            changed=copy.deepcopy(rows);changed[1]['renderer'][field]=value
            with self.assertRaises(ValueError):compare(changed,identity)
        changed=copy.deepcopy(rows);changed[1]['effective']['policy']='inherited'
        with self.assertRaises(ValueError):compare(changed,identity)
if __name__=='__main__':unittest.main()

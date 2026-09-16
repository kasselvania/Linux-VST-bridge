import copy,json,pathlib,tempfile,unittest
from identity import *
from test_fixtures import fake_package
class PackageTests(unittest.TestCase):
    def test_exact_set_seal_source_and_every_file(self):
        with tempfile.TemporaryDirectory() as t:
            root=pathlib.Path(t)/'package';m,seal=fake_package(root)
            self.assertEqual(verify_package(root,seal,m['source']),m)
            for name in REQUIRED:
                p=root/name;old=p.read_bytes();p.chmod(0o600);p.write_bytes(b'changed');p.chmod(0o400)
                with self.subTest(file=name),self.assertRaises(ValueError):verify_package(root,seal,m['source'])
                p.chmod(0o600);p.write_bytes(old);p.chmod(0o400)
            (root/'extra').write_text('extra')
            with self.assertRaises(ValueError):verify_package(root,seal,m['source'])
            (root/'extra').unlink()
            with self.assertRaises(ValueError):verify_package(root,'0'*64,m['source'])
            with self.assertRaises(ValueError):verify_package(root,seal,{'head':'c'*40,'tree':'d'*40})
    def test_caller_cannot_shrink_required_manifest_or_reseal_changed_source(self):
        with tempfile.TemporaryDirectory() as t:
            root=pathlib.Path(t)/'package';m,seal=fake_package(root);p=root/MANIFEST;p.chmod(0o600)
            del m['files']['supervise.py'];p.write_bytes(canonical(m));p.chmod(0o400)
            with self.assertRaises(ValueError):verify_package(root,digest(p),m['source'])
            with self.assertRaises(ValueError):verify_package(root,seal,m['source'])
    def test_duplicate_schema_symlink_and_atomic_no_replace(self):
        with tempfile.TemporaryDirectory() as t:
            root=pathlib.Path(t)/'package';m,seal=fake_package(root)
            p=root/'run.py';p.unlink();p.symlink_to(root/'session.py')
            with self.assertRaises(ValueError):verify_package(root,seal,m['source'])
            out=pathlib.Path(t)/'receipt';atomic_new(out,{'value':1});before=out.read_bytes()
            with self.assertRaises(FileExistsError):atomic_new(out,{'value':2})
            self.assertEqual(out.read_bytes(),before);self.assertFalse(list(pathlib.Path(t).glob('.is3-*')))
            p=pathlib.Path(t)/'duplicate';p.write_text('{"a":1,"a":2}')
            with self.assertRaises(ValueError):read_json(p)
    def test_runner_selected_exactly_not_first_or_string_id(self):
        with tempfile.TemporaryDirectory() as t:
            p=pathlib.Path(t)/'proton';p.write_bytes(b'owned')
            r={'id':'pinned','version':'v','entry_point':str(p),'proton':str(p),'files':[{'path':str(p),'sha256':digest(p)}]}
            expected=runner_identity(r)
            def registry(rs):return {'classes':{str(i):{'registration':{'environment':{'runner':x}}} for i,x in enumerate(rs)}}
            self.assertEqual(select_runner(registry([dict(r,id='other'),r,copy.deepcopy(r)]),expected),r)
            with self.assertRaises(ValueError):select_runner(registry([dict(r,version='changed')]),expected)
            with self.assertRaises(ValueError):select_runner(registry([r,dict(r,version='changed')]),expected)
            p.write_bytes(b'changed')
            with self.assertRaises(ValueError):select_runner(registry([r]),expected)

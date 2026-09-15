import importlib.util,json,pathlib,tempfile,unittest,zipfile,hashlib
from native_builder import build,SDK,SDK_RUNTIME,run
class BuildContract(unittest.TestCase):
    def request(self,root,files,extra=None):
        kit=root/'kit.zip'
        recipe=dict(schema=1,source_commit='a'*40,sdk=SDK,sdk_runtime=SDK_RUNTIME,files={k:hashlib.sha256(v).hexdigest() for k,v in files.items()})
        with zipfile.ZipFile(kit,'w') as z:
            z.writestr('recipe.json',json.dumps(recipe))
            for name,data in files.items():z.writestr(name,data)
            if extra:z.writestr(*extra)
        return dict(directory=str(root/'work'),kit=str(kit),kit_sha256=hashlib.sha256(kit.read_bytes()).hexdigest())
    def test_path_traversal_refused_before_compiler(self):
        with tempfile.TemporaryDirectory() as d:
            r=pathlib.Path(d);request=self.request(r,{'../escape':b'x'})
            with self.assertRaises(AssertionError):build(request,lambda *a:None)
            self.assertFalse((r/'escape').exists())
    def test_extra_unbound_source_refused(self):
        with tempfile.TemporaryDirectory() as d:
            r=pathlib.Path(d);request=self.request(r,{'libap2_backend.a':b'x'},('foreign',b'y'))
            with self.assertRaises(AssertionError):build(request,lambda *a:None)
    def test_source_digest_mismatch_refused(self):
        with tempfile.TemporaryDirectory() as d:
            r=pathlib.Path(d);request=self.request(r,{'libap2_backend.a':b'x'})
            request['kit_sha256']='0'*64
            with self.assertRaises(AssertionError):build(request,lambda *a:None)
    def test_bounded_drain_after_retention(self):
        import sys
        data,dropped=run([sys.executable,'-c','print("x"*70000)'],10)
        self.assertEqual(len(data),32768);self.assertGreater(dropped,0)
    def test_generator_is_shared_production_descriptor_owner(self):
        p=pathlib.Path(__file__).resolve().parents[2]/'bridge-manager/src/preparation/build.rs'
        self.assertIn('include_str!("../../../tools/ap8_descriptor.py")',p.read_text())
if __name__=='__main__':unittest.main()

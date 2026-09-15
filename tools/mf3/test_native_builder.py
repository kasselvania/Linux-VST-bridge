import importlib.util,json,pathlib,tempfile,unittest,zipfile,hashlib
from native_builder import build,SDK,SDK_RUNTIME,run
class BuildContract(unittest.TestCase):
    def request(self,root,files,extra=None):
        kit=root/'kit.zip'
        recipe=dict(schema=2,source_commit='a'*40,sdk=SDK,sdk_runtime=SDK_RUNTIME,files={k:hashlib.sha256(v).hexdigest() for k,v in files.items()})
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
    def test_generators_are_kit_owned_and_not_manager_embedded(self):
        p=pathlib.Path(__file__).resolve().parents[2]/'bridge-manager/src/preparation/build.rs'
        self.assertNotIn('include_str!("../../../tools/ap8_descriptor.py")',p.read_text())
        self.assertIn('include_str!("../../../tools/mf3/kit_entry.py")',p.read_text())
    def test_mixed_kit_and_generator_fail_closed(self):
        from kit_entry import load_generators,BUILDER,GENERATOR
        with tempfile.TemporaryDirectory() as d:
            r=pathlib.Path(d);q=self.request(r,{BUILDER:b'first builder',GENERATOR:b'first generator'})
            recipe,loaded=load_generators(q)
            self.assertEqual(loaded,[b'first builder',b'first generator'])
            with zipfile.ZipFile(r/'other.zip','w') as z:
                z.writestr('recipe.json',json.dumps(recipe));z.writestr(BUILDER,b'changed builder');z.writestr(GENERATOR,b'first generator')
            q['kit']=str(r/'other.zip');q['kit_sha256']=hashlib.sha256((r/'other.zip').read_bytes()).hexdigest()
            with self.assertRaisesRegex(AssertionError,'generator_changed'):load_generators(q)
    def test_legacy_kit_cannot_borrow_new_generators(self):
        from kit_entry import load_generators
        with tempfile.TemporaryDirectory() as d:
            r=pathlib.Path(d);q=self.request(r,{'libap2_backend.a':b'x'})
            with zipfile.ZipFile(r/'old.zip','w') as z:z.writestr('recipe.json',json.dumps({'schema':1}))
            q['kit']=str(r/'old.zip');q['kit_sha256']=hashlib.sha256((r/'old.zip').read_bytes()).hexdigest()
            with self.assertRaisesRegex(AssertionError,'generator_identity_missing'):load_generators(q)
if __name__=='__main__':unittest.main()

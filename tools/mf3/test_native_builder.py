import importlib.util,json,pathlib,tempfile,unittest,zipfile,hashlib
from unittest.mock import patch
from native_builder import build,SDK,SDK_RUNTIME,run
class BuildContract(unittest.TestCase):
    def prebuilt_request(self,root,descriptor='descriptor',module='ab'*32,native=b'\x7fELFtest-proxy'):
        class_id='01'*16
        name='prebuilt/'+class_id+'-'+module+'.so'
        sha=lambda b:hashlib.sha256(b).hexdigest()
        index=dict(schema=1,proxies=[dict(class_id=class_id,module_sha256=module,file=name,
            descriptor_sha256=sha(descriptor.encode()),native_sha256=sha(native))])
        files={name:native,'prebuilt/index.json':json.dumps(index).encode()}
        recipe=dict(schema=3,source_commit='a'*40,sdk=SDK,sdk_runtime=SDK_RUNTIME,
            files={k:sha(v) for k,v in files.items()})
        kit=root/'kit.zip'
        with zipfile.ZipFile(kit,'w') as z:
            z.writestr('recipe.json',json.dumps(recipe))
            for name,data in files.items():z.writestr(name,data)
        inspection=root/'inspection.json';inspection.write_text(json.dumps({'records':[]}))
        return dict(directory=str(root/'work'),kit=str(kit),kit_sha256=sha(kit.read_bytes()),
            inspection=str(inspection),class_id=class_id,module_sha256=module)
    def test_prebuilt_install_is_offline_and_never_invokes_a_compiler(self):
        with tempfile.TemporaryDirectory() as d,patch('native_builder.subprocess.Popen',side_effect=AssertionError('process forbidden')):
            root=pathlib.Path(d);request=self.prebuilt_request(root)
            result=build(request,lambda *a:'descriptor')
            self.assertEqual((root/'work/native.so').read_bytes(),b'\x7fELFtest-proxy')
            self.assertEqual(result['delivery'],'prebuilt')
            self.assertIsNone(result['sdk_runtime'])
    def test_another_module_or_changed_metadata_cannot_use_shipped_proxy(self):
        for changed in ('module','metadata'):
            with tempfile.TemporaryDirectory() as d,patch('native_builder.subprocess.Popen',side_effect=AssertionError('process forbidden')):
                root=pathlib.Path(d);request=self.prebuilt_request(root)
                if changed=='module':request['module_sha256']='cd'*32
                with self.assertRaisesRegex(ValueError,'unavailable' if changed=='module' else 'metadata_mismatch'):
                    build(request,lambda *a:'changed descriptor')
                self.assertFalse((root/'work/native.so').exists())
    def test_prebuilt_non_elf_payload_is_refused(self):
        with tempfile.TemporaryDirectory() as d:
            root=pathlib.Path(d);request=self.prebuilt_request(root,native=b'foreign payload')
            with self.assertRaises(AssertionError):build(request,lambda *a:'descriptor')
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

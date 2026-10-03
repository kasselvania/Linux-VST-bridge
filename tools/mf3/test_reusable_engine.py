"""No vendor files. Unknown classes are chosen after the kit is assembled."""
import hashlib, json, os, pathlib, subprocess, sys, tempfile, unittest, uuid, zipfile
from unittest.mock import patch
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))
from ap8_descriptor import runtime_descriptor
from native_builder import build, SDK, SDK_RUNTIME
from prebuilt_info import maximum_bridge_frames

def sha(data): return hashlib.sha256(data).hexdigest()
def kit(root, engine=b'\x7fELFunit-test-engine'):
    index=dict(schema=3,engine='prebuilt/engine.so',engine_sha256=sha(engine),
               descriptor_schema=1,maximum_bridge_frames=1024,native_sources={})
    files={'prebuilt/engine.so':engine,'prebuilt/index.json':json.dumps(index).encode(),
           'tools/mf3/native_builder.py':pathlib.Path(__file__).with_name('native_builder.py').read_bytes(),
           'tools/ap8_descriptor.py':(pathlib.Path(__file__).resolve().parents[1]/'ap8_descriptor.py').read_bytes()}
    recipe=dict(schema=4,source_commit='a'*40,sdk=SDK,sdk_runtime=SDK_RUNTIME,
                files={k:sha(v) for k,v in files.items()})
    path=root/'kit.zip'
    with zipfile.ZipFile(path,'w') as z:
        for name,data in files.items():z.writestr(name,data)
        z.writestr('recipe.json',json.dumps(recipe))
    return path

def records(cid, effect=False, value=0.25):
    buses=[dict(state='ap8_bus',media=0,direction=1,index=0,channels=2,type=0,flags=1,arrangement=3,name='Output')]
    if effect:buses.insert(0,{**buses[0],'direction':0,'name':'Input'})
    return buses+[
        dict(state='ap8_inspected',float32_result=0),
        dict(state='ap12_class',class_id=cid,name='Unseen Effect' if effect else 'Unseen Instrument',vendor='Reference',
             version='1.0',subcategories='Fx' if effect else 'Instrument|Synth',metadata_tier='factory_2'),
        dict(state='ap8_parameter_count',count=1),
        dict(state='ap8_parameters',parameters=[[7,'Level','',0,1,0.5,value]])]

def prepare(root, archive, cid, module, effect=False, value=0.25, loader=False):
    inspection=root/'inspection.json';inspection.write_text(json.dumps({'records':records(cid,effect,value)}))
    request=dict(directory=str(root/'prepared'),kit=str(archive),kit_sha256=sha(archive.read_bytes()),
                 inspection=str(inspection),class_id=cid,module_sha256=module)
    if loader:
        result=subprocess.run([sys.executable,'-I',str(pathlib.Path(__file__).with_name('kit_entry.py'))],
            input=json.dumps(request),text=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE,
            env={**os.environ,'PATH':'/no-compiler-or-sdk'},check=True,timeout=10)
        return json.loads(result.stdout)
    return build(request,runtime_descriptor)

class ReusableEngine(unittest.TestCase):
    def test_manager_loader_uses_kit_owned_generators_without_build_tools(self):
        with tempfile.TemporaryDirectory() as d:
            root=pathlib.Path(d);archive=kit(root)
            result=prepare(root,archive,uuid.uuid4().hex.upper(),'ab'*32,loader=True)
            self.assertEqual(result['delivery'],'reusable_engine')
            self.assertEqual(result['kit_sha256'],sha(archive.read_bytes()))
            self.assertEqual(sha((root/'prepared/native.so').read_bytes()),result['native_sha256'])
            self.assertEqual(sha((root/'prepared/plugin-descriptor.json').read_bytes()),result['descriptor_sha256'])

    def test_unseen_instrument_effect_and_update_use_identical_engine_without_compiler(self):
        with tempfile.TemporaryDirectory() as d, patch('native_builder.subprocess.Popen',side_effect=AssertionError('compiler forbidden')):
            root=pathlib.Path(d);archive=kit(root)
            # Identities are deliberately chosen after freezing all kit bytes.
            ids=[uuid.uuid4().hex.upper() for _ in range(2)]
            engine_hashes=[];descriptors=[]
            for i,(cid,mod,effect,value) in enumerate([(ids[0],'ab'*32,False,0.25),(ids[1],'cd'*32,True,0.6),(ids[0],'ef'*32,False,0.75)]):
                work=root/str(i);work.mkdir();result=prepare(work,archive,cid,mod,effect,value)
                descriptor=json.loads((work/'prepared/plugin-descriptor.json').read_bytes())
                self.assertEqual(descriptor['class_id'],cid);self.assertEqual(descriptor['parameters'][0]['initial'],value)
                self.assertEqual(maximum_bridge_frames(dict(kit=str(archive),class_id=cid,module_sha256=mod,native_sha256=result['native_sha256'])),1024)
                engine_hashes.append(sha((work/'prepared/native.so').read_bytes()));descriptors.append(result['descriptor_sha256'])
            self.assertEqual(len(set(engine_hashes)),1);self.assertEqual(len(set(descriptors)),3)

    def test_changed_engine_and_unsupported_bus_are_not_prepared(self):
        with tempfile.TemporaryDirectory() as d:
            root=pathlib.Path(d);archive=kit(root,b'not ELF');work=root/'work';work.mkdir()
            with self.assertRaises(AssertionError):prepare(work,archive,'01'*16,'ab'*32)
            self.assertFalse((work/'prepared/native.so').exists())
        rows=records('01'*16);rows[0]['channels']=6
        with self.assertRaisesRegex(ValueError,'stereo'):runtime_descriptor(rows,'01'*16,'ab'*32,'cd'*32)

if __name__=='__main__':unittest.main()

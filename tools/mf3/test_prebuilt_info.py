import hashlib, json, pathlib, tempfile, unittest, zipfile
from prebuilt_info import maximum_bridge_frames, capability

class PrebuiltCapabilities(unittest.TestCase):
    def fixture(self, root, schema=2, maximum=1024, corrupt=False):
        native = hashlib.sha256(b'native').hexdigest()
        index = dict(schema=schema, proxies=[dict(class_id='01'*16,
            module_sha256='ab'*32, native_sha256=native, file='prebuilt/proxy.so')])
        if schema == 2:
            index['proxies'][0]['maximum_bridge_frames'] = maximum
        data = json.dumps(index).encode()
        recipe = dict(schema=3, files={'prebuilt/index.json':
            '00'*32 if corrupt else hashlib.sha256(data).hexdigest(), 'prebuilt/proxy.so':native})
        kit = root/'kit.zip'
        with zipfile.ZipFile(kit, 'w') as archive:
            archive.writestr('recipe.json', json.dumps(recipe))
            archive.writestr('prebuilt/index.json', data)
        return dict(kit=str(kit), class_id='01'*16, module_sha256='ab'*32, native_sha256=native)

    def test_legacy_index_cannot_acquire_new_buffering(self):
        with tempfile.TemporaryDirectory() as directory:
            request = self.fixture(pathlib.Path(directory), schema=1)
            self.assertEqual(maximum_bridge_frames(request), 512)

    def test_exact_native_and_module_are_required(self):
        with tempfile.TemporaryDirectory() as directory:
            request = self.fixture(pathlib.Path(directory))
            self.assertEqual(maximum_bridge_frames(request), 1024)
            for key, value in [('native_sha256','cd'*32), ('module_sha256','cd'*32), ('class_id','02'*16)]:
                self.assertIsNone(maximum_bridge_frames({**request,key:value}))

    def test_new_delivery_requires_declaration_and_exact_paired_executables(self):
        for supported in (False, True):
            with tempfile.TemporaryDirectory() as directory:
                kit = pathlib.Path(directory)/'kit.zip'
                native, host = 'ab'*32, 'cd'*32
                index = dict(schema=3, engine='prebuilt/engine.so', engine_sha256=native,
                    descriptor_schema=1, maximum_bridge_frames=1024, native_sources={})
                if supported:
                    index['audio_completion_contract'] = 1
                data = json.dumps(index).encode()
                recipe = dict(schema=4, files={'prebuilt/index.json':hashlib.sha256(data).hexdigest(),
                    'prebuilt/engine.so':native,'runtime/host.exe':host})
                with zipfile.ZipFile(kit, 'w') as archive:
                    archive.writestr('recipe.json', json.dumps(recipe))
                    archive.writestr('prebuilt/index.json', data)
                request = dict(kit=str(kit),native_sha256=native,host_sha256=host)
                self.assertEqual(capability(request, 'audio_completion_contract'), 1 if supported else None)
                for name in ('native_sha256','host_sha256'):
                    self.assertIsNone(capability({**request, name:'ef'*32}, 'audio_completion_contract'))

    def test_unknown_capability_and_changed_index_refuse(self):
        for options in (dict(maximum=2048), dict(corrupt=True)):
            with tempfile.TemporaryDirectory() as directory:
                request = self.fixture(pathlib.Path(directory), **options)
                with self.assertRaises(AssertionError):
                    maximum_bridge_frames(request)

if __name__ == '__main__':
    unittest.main()

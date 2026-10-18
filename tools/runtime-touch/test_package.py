"""Bind the binary, consumed sources and original GE driver before distribution."""
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import build
import package

class PackageTest(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
        self.root=Path(self.tmp.name);self.built=self.root/'build';self.built.mkdir()
        def put(name,data):
            p=self.built/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(data);return p
        self.put=put
        self.sources={name:build.sha(put('wine/dlls/winex11.drv/'+name,('changed '+name).encode())) for name in ('mouse.c','touch_message_flags.h')}
        put('wine/COPYING.LIB',b'license');put('wine/unchanged.c',b'unchanged source')
        for name in ['build.py','build.sh','apply-ge.sh','test_touch_flags.c',*build.PATCHES]:put(name,('recipe '+name).encode())
        inputs={name:(url,build.sha(put('inputs/'+name,name.encode()))) for name,(url,_) in build.INPUTS.items()}
        self.addCleanup(patch.stopall);patch.object(build,'INPUTS',inputs).start()
        data=bytearray(64);data[:6]=b'\x7fELF\x02\x01';data[18]=62
        self.driver=put('obj64/dlls/winex11.drv/winex11.so',data)
        self.record=dict(wine=build.WINE,ge=build.GE,staging=build.STAGING,sdk=build.SDK,mapping_test='passed',
            inputs={n:h for n,(_,h) in inputs.items()},touch_sources=self.sources,
            recipe_sha256=build.sha(self.built/'build.py'),build_script_sha256=build.sha(self.built/'build.sh'),
            ge_script_sha256=build.sha(self.built/'apply-ge.sh'),patches={n:build.sha(self.built/n) for n in build.PATCHES},
            artifact=dict(path=str(self.driver.relative_to(self.built)),sha256=build.sha(self.driver),size=64))
        self.record_path=put('build-manifest.json',json.dumps(self.record).encode())
        self.ge=self.root/'ge.tar.gz'
        with tarfile.open(self.ge,'w:gz') as tar:package.add_bytes(tar,package.GE_ROOT+'/'+package.TARGET,b'original')
        patch.object(package,'GE_SHA',build.sha(self.ge)).start()
        patch.object(package,'PREIMAGE',package.hashlib.sha256(b'original').hexdigest()).start()
    def run_package(self):return package.package(self.built,self.ge,self.root/'package')
    def test_source_and_exact_replacement_accompany_binary(self):
        result=self.run_package()
        self.assertEqual(result['manifest']['files'][0]['original_sha256'],package.PREIMAGE)
        with tarfile.open(self.root/'package/x11-touch/wine-source.tar.gz') as tar:
            self.assertEqual(tar.extractfile('wine/unchanged.c').read(),b'unchanged source')
            self.assertIn('upstream/ge.tar.gz',tar.getnames())
            self.assertIn('build.sh',tar.getnames())
    def test_changed_binary_refused(self):
        self.driver.write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError,'Built driver changed'):self.run_package()
    def test_changed_touch_source_refused(self):
        self.put('wine/dlls/winex11.drv/mouse.c',b'changed again')
        with self.assertRaisesRegex(ValueError,'Consumed touch source changed'):self.run_package()
    def test_changed_upstream_source_refused(self):
        self.put('inputs/ge.tar.gz',b'changed')
        with self.assertRaisesRegex(ValueError,'Upstream input changed'):self.run_package()
    def test_missing_source_binding_refused(self):
        self.record['touch_sources']={};self.record_path.write_text(json.dumps(self.record))
        with self.assertRaisesRegex(ValueError,'Build identity changed'):self.run_package()
    def test_wrong_ge_preimage_refused(self):
        with patch.object(package,'PREIMAGE','0'*64):
            with self.assertRaisesRegex(ValueError,'GE driver preimage changed'):self.run_package()

if __name__=='__main__':unittest.main()

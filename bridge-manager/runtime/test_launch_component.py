"""Selected kit execution binding; no vendor/application launch."""
import hashlib
import json
import pathlib
import tempfile
import unittest
import zipfile
import session


class LaunchComponentTests(unittest.TestCase):
    def fixture(self, root):
        managed=root/'managed';software=managed/'software';software.mkdir(parents=True)
        binary=b'MZfirst-party execution fixture'
        origin=b'{"schema":1}'
        values={'runtime/installer-launch.exe':binary,'runtime/installer-launch-source.json':origin}
        recipe={'schema':4,'files':{n:hashlib.sha256(b).hexdigest() for n,b in values.items()}}
        kit=software/'kit.zip'
        with zipfile.ZipFile(kit,'w') as z:
            for n,b in values.items():z.writestr(n,b)
            z.writestr('recipe.json',json.dumps(recipe))
        kit.chmod(0o400);sha=hashlib.sha256(kit.read_bytes()).hexdigest()
        selected={'installer_launch':None,'preparation_kit':{'path':str(kit),'sha256':sha}}
        (managed/'software.json').write_text(json.dumps(selected))
        adapter=software/'preparation-kits'/sha/'installer-launch.exe'
        adapter.parent.mkdir(parents=True);adapter.write_bytes(binary);adapter.chmod(0o400)
        metadata=adapter.with_name('installer-launch-source.json');metadata.write_bytes(origin);metadata.chmod(0o400)
        return managed,selected,adapter

    def test_complete_set_is_derived_without_changing_selection_and_rejects_changed_bytes(self):
        with tempfile.TemporaryDirectory() as tmp:
            managed,selected,adapter=self.fixture(pathlib.Path(tmp).resolve())
            before=(managed/'software.json').read_bytes()
            complete=session.renderer_launch_software(managed)
            self.assertEqual(complete,{**selected,'installer_launch':{
                'path':str(adapter),'sha256':hashlib.sha256(adapter.read_bytes()).hexdigest()}})
            self.assertEqual((managed/'software.json').read_bytes(),before)
            adapter.chmod(0o600);adapter.write_bytes(b'changed');adapter.chmod(0o400)
            with self.assertRaises(Exception):session.renderer_launch_software(managed)

    def test_foreign_staged_image_and_unbound_kit_are_not_a_launch_component(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp).resolve();managed,selected,adapter=self.fixture(root)
            other=root/'same-bytes.exe';other.write_bytes(adapter.read_bytes());adapter.unlink();adapter.symlink_to(other)
            with self.assertRaisesRegex(ValueError,'component_authority'):
                session.renderer_launch_software(managed)
            selected['preparation_kit']['sha256']='ff'*32
            (managed/'software.json').write_text(json.dumps(selected))
            with self.assertRaises(Exception):session.renderer_launch_software(managed)


if __name__=='__main__':unittest.main()

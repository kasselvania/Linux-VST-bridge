"""Uninstall through the owned installer loop: only the closed manager shapes launch."""
import json,pathlib,signal,subprocess,sys,tempfile,unittest
from unittest.mock import patch
import session
from test_installer import Scope

def spec_for(root, fmt, image, registration='ab'*32, product_code=None, arguments=()):
    return {'schema':2,'operation':'cd'*16,'format':fmt,'report':str(root/'result.json'),'preserve_existing_prefix':True,
        'environment':{'root':str(root),'runner':{'entry_point':'fixed-runner','proton':str(root/'proton/proton'),'files':[]}},
        'installer':{'path':str(image),'sha256':session.hashlib.sha256(image.read_bytes()).hexdigest()},
        'uninstall':{'schema':1,'registration':registration,'product_code':product_code,'arguments':list(arguments)}}

class UninstallTests(unittest.TestCase):
    def tree(self, tmp):
        root=pathlib.Path(tmp).resolve();(root/'operation.lock').touch();(root/'home').mkdir()
        drive=root/'compatdata/pfx/drive_c';(drive/'Program Files/Vendor').mkdir(parents=True);(drive/'windows/system32').mkdir(parents=True)
        (root/'compatdata/pfx/system.reg').touch()
        uninstaller=drive/'Program Files/Vendor/unins000.exe';uninstaller.write_bytes(b'MZ uninstaller')
        (drive/'windows/system32/cmd.exe').write_bytes(b'MZ cmd')
        msiexec=root/'proton/files/lib/wine/x86_64-windows/msiexec.exe';msiexec.parent.mkdir(parents=True);msiexec.write_bytes(b'MZ msiexec')
        return root,uninstaller,msiexec

    def test_only_closed_shapes_validate(self):
        with tempfile.TemporaryDirectory() as tmp:
            root,uninstaller,msiexec=self.tree(tmp)
            code='{0000000A-1111-2222-3333-44444444444B}'
            session.uninstall_validate(spec_for(root,'uninstall_executable',uninstaller,arguments=['/uninstall']),root)
            session.uninstall_validate(spec_for(root,'uninstall_msi',msiexec,product_code=code),root)
            refused=[
                spec_for(root,'uninstall_executable',root/'compatdata/pfx/drive_c/windows/system32/cmd.exe'),
                spec_for(root,'uninstall_executable',msiexec),
                spec_for(root,'uninstall_executable',uninstaller,product_code=code),
                spec_for(root,'uninstall_executable',uninstaller,arguments=['bad\nline']),
                spec_for(root,'uninstall_executable',uninstaller,registration='not-a-digest'),
                spec_for(root,'uninstall_msi',msiexec,product_code='{0000-1111}'),
                spec_for(root,'uninstall_msi',msiexec,product_code=code.lower()),
                spec_for(root,'uninstall_msi',msiexec,product_code=code,arguments=['/qn']),
                spec_for(root,'uninstall_msi',uninstaller,product_code=code),
            ]
            extra=spec_for(root,'uninstall_executable',uninstaller);extra['uninstall']['command']='anything';refused.append(extra)
            for spec in refused:
                with self.subTest(spec=spec['uninstall'],image=spec['installer']['path']),self.assertRaises(ValueError):
                    session.uninstall_validate(spec,root)

    @unittest.skipUnless(sys.platform.startswith('linux'),'Linux supervisor subreaper')
    def test_owned_loop_launches_the_registered_program_or_exact_msi_removal(self):
        code='{0000000A-1111-2222-3333-44444444444B}'
        for fmt in ('uninstall_executable','uninstall_msi'):
            with self.subTest(fmt=fmt),tempfile.TemporaryDirectory() as tmp:
                root,uninstaller,msiexec=self.tree(tmp)
                spec=spec_for(root,fmt,uninstaller,arguments=['/uninstall']) if fmt=='uninstall_executable' else spec_for(root,fmt,msiexec,product_code=code)
                real_popen=subprocess.Popen;children=[];calls=[]
                def launch(args,**kwargs):
                    calls.append(args)
                    p=real_popen([sys.executable,'-c','print("ready")'],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
                    children.append(p);return p
                old=[signal.getsignal(s) for s in (signal.SIGTERM,signal.SIGINT)]
                try:
                    with patch.object(session,'CompanionCgroup',return_value=Scope()),patch.object(session.subprocess,'Popen',side_effect=launch),patch.object(session,'environment',return_value={}):
                        self.assertTrue(session.install(spec))
                    v=json.loads((root/'result.json').read_text())
                    self.assertEqual(v['state'],'completed');self.assertTrue(v['cleanup_confirmed']);self.assertEqual(v['owned_live'],0)
                    base=['fixed-runner','--verb=run','--',str(root/'proton/proton'),'runinprefix']
                    if fmt=='uninstall_executable':self.assertEqual(calls[1],base+[str(uninstaller),'/uninstall'])
                    else:self.assertEqual(calls[1][:len(base)+3],base+['msiexec','/x',code])
                finally:
                    for sig,handler in zip((signal.SIGTERM,signal.SIGINT),old):signal.signal(sig,handler)
                    for p in children:
                        if p.poll() is None:p.kill();p.wait()
                    session.ctypes.CDLL(None).prctl(36,0,0,0,0)

if __name__=='__main__':unittest.main()

"""An app an installer starts by itself must not stay running in place of its
managed first launch; only exact profiled images are ended, after the installer."""
import json,os,pathlib,signal,subprocess,sys,tempfile,time,unittest
from unittest.mock import patch
import session
from test_installer import Scope

PROFILE={'profile':'fixture_app','executable':'App.exe','resources':['resources/app.asar','app.pak']}
CREATE='1.000:0020:0024:trace:process:CreateProcessInternalW app L"C:\\\\Program Files\\\\Vendor\\\\App.exe" cmdline L"\\"C:\\\\Program Files\\\\Vendor\\\\App.exe\\""\n'
STARTED='1.001:0020:0024:trace:process:CreateProcessInternalW started process pid 0030 tid 0034\n'

def app_tree(root,*,resources=True):
    directory=root/'compatdata/pfx/drive_c/Program Files/Vendor';(directory/'resources').mkdir(parents=True)
    (directory/'App.exe').write_bytes(b'MZ'+bytes(4094))
    if resources:
        for name in PROFILE['resources']:(directory/name).write_bytes(b'r')
    return directory/'App.exe'

class ProfileDataTests(unittest.TestCase):
    def test_manager_profile_data_is_closed(self):
        self.assertEqual(session.installer_first_launch_profiles({}),[])
        self.assertEqual(session.installer_first_launch_profiles({'managed_first_launch':[PROFILE]}),[PROFILE])
        for bad in [{'profile':'x','executable':'App.exe'},dict(PROFILE,extra=1),dict(PROFILE,executable='../App.exe'),
                dict(PROFILE,executable='dir\\App.exe'),dict(PROFILE,executable='App.dll'),dict(PROFILE,profile='Not Valid'),
                dict(PROFILE,resources=[]),dict(PROFILE,resources=['../x']),dict(PROFILE,resources=['/abs']),dict(PROFILE,resources=['a//b'])]:
            with self.subTest(bad=bad),self.assertRaises(ValueError):session.installer_first_launch_profiles({'managed_first_launch':[bad]})
        with self.assertRaises(ValueError):session.installer_first_launch_profiles({'managed_first_launch':{}})

class ImageMatchTests(unittest.TestCase):
    def trace(self,root,lines):
        trace=session.InstallerWindowsTrace({'path':str(root/'installer.exe'),'sha256':'0'*64},root);trace.begin()
        for line in lines:trace.feed(line.encode())
        return trace
    def test_only_running_profiled_image_with_its_resources_in_this_prefix(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp).resolve();exe=app_tree(root);st=exe.stat()
            found=session.installer_started_profiled_images(self.trace(root,[CREATE,STARTED]),[PROFILE])
            self.assertEqual(found,{(os.major(st.st_dev),os.minor(st.st_dev),st.st_ino):{'profile':'fixture_app','image':'Program Files/Vendor/App.exe'}})
            # Windows names are case-insensitive; the file found must still exist.
            self.assertEqual(session.installer_started_profiled_images(self.trace(root,[CREATE,STARTED]),[dict(PROFILE,executable='APP.EXE')]).__len__(),1)
            exited='1.002:0030:0034:trace:process:NtTerminateProcess handle 0xffffffffffffffff, exit_code 0, process_exiting 1.\n'
            self.assertEqual(session.installer_started_profiled_images(self.trace(root,[CREATE,STARTED,exited]),[PROFILE]),{})
            self.assertEqual(session.installer_started_profiled_images(self.trace(root,[CREATE,STARTED]),[]),{})
            other=dict(PROFILE,executable='Other.exe')
            self.assertEqual(session.installer_started_profiled_images(self.trace(root,[CREATE,STARTED]),[other]),{})
    def test_home_reached_through_a_symlink_still_finds_the_app(self):
        with tempfile.TemporaryDirectory() as tmp:
            real=pathlib.Path(tmp).resolve()/'var-home';real.mkdir();app_tree(real)
            link=pathlib.Path(tmp).resolve()/'home';link.symlink_to(real)
            self.assertEqual(len(session.installer_started_profiled_images(self.trace(link,[CREATE,STARTED]),[PROFILE])),1)
    def test_installer_root_success_is_its_own_traced_exit_not_the_runner(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp).resolve();artifact=root/'installer.exe';artifact.write_bytes(b'x')
            windows='Z:'+str(artifact).replace('/','\\\\')
            create=f'0.9:0010:0014:trace:process:CreateProcessInternalW app L"{windows}" cmdline L"x"\n'
            started='0.91:0010:0014:trace:process:CreateProcessInternalW started process pid 0020 tid 0024\n'
            def exit_line(code):return f'1.5:0020:0024:trace:process:NtTerminateProcess handle 0xffffffffffffffff, exit_code {code}, process_exiting 1.\n'
            self.assertFalse(self.trace(root,[create,started]).root_succeeded())
            self.assertTrue(self.trace(root,[create,started,exit_line(0)]).root_succeeded())
            self.assertFalse(self.trace(root,[create,started,exit_line(3)]).root_succeeded())
            # A child exiting 0 is not the installer finishing.
            self.assertFalse(self.trace(root,[create,started,CREATE.replace('0020:0024','0020:0024'),
                '1.0:0020:0024:trace:process:CreateProcessInternalW started process pid 0030 tid 0034\n',
                '1.2:0030:0034:trace:process:NtTerminateProcess handle 0xffffffffffffffff, exit_code 0, process_exiting 1.\n']).root_succeeded())
            unbound=self.trace(root,[create,started,exit_line(0)]);unbound.binding={'status':'unavailable'}
            self.assertFalse(unbound.root_succeeded())
    def test_missing_resource_alias_or_outside_prefix_is_not_the_profiled_app(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp).resolve();exe=app_tree(root,resources=False)
            self.assertEqual(session.installer_started_profiled_images(self.trace(root,[CREATE,STARTED]),[PROFILE]),{})
            for name in PROFILE['resources']:(exe.parent/name).write_bytes(b'r')
            (exe.parent/'app.pak').unlink();(exe.parent/'app.pak').symlink_to(exe.parent/'resources/app.asar')
            self.assertEqual(session.installer_started_profiled_images(self.trace(root,[CREATE,STARTED]),[PROFILE]),{})
            outside=CREATE.replace('C:\\\\Program Files','Z:\\\\tmp')
            self.assertNotEqual(outside,CREATE)
            self.assertEqual(session.installer_started_profiled_images(self.trace(root,[outside,STARTED]),[PROFILE]),{})

@unittest.skipUnless(sys.platform.startswith('linux'),'/proc maps')
class MappedIdentityTests(unittest.TestCase):
    def test_identity_is_how_maps_names_this_file_not_stat(self):
        # On Btrfs stat's device differs from maps' (stat 0:35, maps 00:21).
        import mmap
        with tempfile.TemporaryDirectory() as tmp:
            path=pathlib.Path(tmp).resolve()/'App With Space.exe';path.write_bytes(b'MZ'+bytes(5000))
            identity=session.mapped_file_identity(path)
            with open(path,'rb') as f:
                m=mmap.mmap(f.fileno(),4096,access=mmap.ACCESS_READ)
                try:lines=[l.split(None,5) for l in pathlib.Path('/proc/self/maps').read_bytes().splitlines()]
                finally:m.close()
            expected={(int(p[3].split(b':')[0],16),int(p[3].split(b':')[1],16),int(p[4])) for p in lines if len(p)==6 and p[5]==os.fsencode(str(path))}
            self.assertEqual({identity},expected)
            empty=pathlib.Path(tmp).resolve()/'empty';empty.touch()
            self.assertIsNone(session.mapped_file_identity(empty))

@unittest.skipUnless(sys.platform.startswith('linux'),'Linux supervisor subreaper and /proc maps')
class InstallerLoopTests(unittest.TestCase):
    def run_installer(self,profiles,*,resources=True):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp).resolve();(root/'operation.lock').touch();(root/'home').mkdir();artifact=root/'installer';artifact.write_bytes(b'fixture')
            exe=app_tree(root,resources=resources)
            spec={'schema':2,'operation':'ab'*16,'environment':{'root':str(root),'runner':{'entry_point':'fixed-runner','proton':'fixed-proton','files':[]}},
                'installer':{'path':str(artifact),'sha256':session.hashlib.sha256(b'fixture').hexdigest()},'format':'pe_executable','report':str(root/'result.json')}
            if profiles is not None:spec['managed_first_launch']=profiles
            # As under the real runner: the trace shows the installer root start
            # the app and then end with status 0, while the runner itself stays
            # until everything it started has ended (pressure-vessel subreaper).
            windows='Z:'+str(artifact).replace('/','\\\\')
            root_create=f'0.900:0010:0014:trace:process:CreateProcessInternalW app L"{windows}" cmdline L"x"\n'
            root_started='0.901:0010:0014:trace:process:CreateProcessInternalW started process pid 0020 tid 0024\n'
            root_exit='1.500:0020:0024:trace:process:NtTerminateProcess handle 0xffffffffffffffff, exit_code 0, process_exiting 1.\n'
            app=('import mmap,sys,time;f=open(sys.argv[1],"rb");m=mmap.mmap(f.fileno(),0,access=mmap.ACCESS_READ);'
                'sys.stdout.write("app running\\n");sys.stdout.flush();time.sleep(6)')
            installer=(f'import os,subprocess,sys,time;sys.stderr.write({root_create+root_started+CREATE+STARTED+root_exit!r});sys.stderr.flush();'
                f'p=subprocess.Popen([sys.executable,"-c",{app!r},{str(exe)!r}],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL);p.wait()')
            real_popen=subprocess.Popen;children=[]
            def launch_owned(args,**kwargs):
                if not children:
                    (root/'compatdata/pfx').mkdir(parents=True,exist_ok=True);(root/'compatdata/pfx/system.reg').touch()
                program='print("prefix ready")' if not children else installer
                child=real_popen([sys.executable,'-c',program],stdout=subprocess.PIPE,stderr=subprocess.PIPE);children.append(child);return child
            scope=Scope();old=[signal.getsignal(s) for s in (signal.SIGTERM,signal.SIGINT)];started=time.monotonic()
            try:
                with patch.object(session,'CompanionCgroup',return_value=scope),patch.object(session.subprocess,'Popen',side_effect=launch_owned),patch.object(session,'environment',return_value={}):
                    ok=session.install(spec)
                return ok,json.loads((root/'result.json').read_text()),time.monotonic()-started
            finally:
                for sig,handler in zip((signal.SIGTERM,signal.SIGINT),old):signal.signal(sig,handler)
                for row in scope.members():
                    try:os.kill(row['pid'],signal.SIGKILL)
                    except ProcessLookupError:pass
                session.ctypes.CDLL(None).prctl(36,0,0,0,0)
    def test_installer_started_profiled_app_is_ended_after_the_installer_finishes(self):
        ok,v,elapsed=self.run_installer([PROFILE])
        self.assertTrue(ok);self.assertEqual(v['state'],'completed');self.assertEqual(v['raw_exit'],0);self.assertEqual(v['owned_live'],0)
        self.assertEqual(v['started_application']['result'],'closed_for_managed_first_launch')
        self.assertEqual(v['started_application']['profiles'],['fixture_app'])
        self.assertEqual(v['started_application']['images'],['Program Files/Vendor/App.exe'])
        self.assertGreaterEqual(v['started_application']['signalled_processes'],1)
        self.assertLess(elapsed,5)
    def test_without_a_matching_profile_the_session_still_waits_for_what_the_installer_started(self):
        for profiles,resources in [(None,True),([],True),([PROFILE],False)]:
            with self.subTest(profiles=profiles,resources=resources):
                ok,v,elapsed=self.run_installer(profiles,resources=resources)
                self.assertTrue(ok);self.assertEqual(v['state'],'completed');self.assertIsNone(v['started_application'])
                self.assertGreater(elapsed,5)

if __name__=='__main__':unittest.main()
